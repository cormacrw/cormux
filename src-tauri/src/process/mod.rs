use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use tokio::sync::{Notify, RwLock};

use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;

const RING_CAP: usize = 500;
/// A line that never ends (`\r`-only progress bars) keeps only its tail.
const FRAGMENT_CAP: usize = 64 * 1024;
/// A log past this starts over; nothing reads it back, it's there for debugging.
const LOG_FILE_CAP: u64 = 10 * 1024 * 1024;
/// How long a stopped session gets to exit before it's killed.
const STOP_GRACE: Duration = Duration::from_millis(200);
const STOP_POLL: Duration = Duration::from_millis(25);

/// Setup and run commands in a PTY, each in its own process group.
#[derive(Clone)]
pub struct ProcessSupervisor {
    env: Arc<RwLock<ShellEnv>>,
    sessions: Arc<Mutex<HashMap<String, RunningApp>>>,
    logs: Arc<Mutex<HashMap<String, Arc<SessionShared>>>>,
    /// Woken on every new line of any session.
    activity: Arc<Notify>,
}

struct SessionShared {
    lines: Mutex<VecDeque<String>>,
    pending: Mutex<Vec<String>>,
    fragment: Mutex<String>,
    log: Mutex<File>,
    log_bytes: AtomicU64,
    exited: Mutex<Option<i32>>,
    killed: AtomicBool,
    activity: Arc<Notify>,
}

struct RunningApp {
    pid: u32,
    shared: Arc<SessionShared>,
    /// The PTY's descriptors, released as soon as the process exits.
    pty: Mutex<Option<Pty>>,
}

struct Pty {
    // Only tests write input, but dropping the writer sends EOF to the app.
    #[cfg_attr(not(test), allow(dead_code))]
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
}

fn log_path(id: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("cormux-pty-{id}.log"))
}

impl ProcessSupervisor {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            env,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(HashMap::new())),
            activity: Arc::new(Notify::new()),
        }
    }

    /// Woken when any session or log gets a new line.
    pub fn activity(&self) -> Arc<Notify> {
        self.activity.clone()
    }

    /// Logs left by earlier runs; a live session's file stays open under it if it's a second app.
    pub fn remove_stale_logs(&self) {
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("cormux-pty-") && name.ends_with(".log") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    pub fn append_log_line(&self, id: &str, line: impl Into<String>) {
        let line = line.into();
        let shared = {
            let mut logs = self.logs.lock().unwrap();
            logs.entry(id.to_string())
                .or_insert_with(|| {
                    let log = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(log_path(id))
                        .unwrap_or_else(|_| {
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(log_path("fallback"))
                                .expect("fallback pty log")
                        });
                    Arc::new(SessionShared::new(log, self.activity.clone()))
                })
                .clone()
        };
        shared.push_line(line);
    }

    #[cfg(test)]
    pub fn spawn(&self, program: &str, args: &[&str]) -> Result<u32> {
        self.spawn_session("default", program, args, None, &[])
    }

    pub fn spawn_session(
        &self,
        id: &str,
        program: &str,
        args: &[&str],
        cwd: Option<&Path>,
        extra_env: &[(String, String)],
    ) -> Result<u32> {
        let _ = self.stop_session(id);

        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| Error::Process(error.to_string()))?;

        let mut cmd = CommandBuilder::new(program);
        for arg in args {
            cmd.arg(*arg);
        }
        if let Some(cwd) = cwd {
            cmd.cwd(cwd);
        }
        {
            if let Ok(env) = self.env.try_read() {
                env.apply_pty(&mut cmd);
            }
        }
        for (key, value) in extra_env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|error| Error::Process(error.to_string()))?;
        let pid = child
            .process_id()
            .ok_or_else(|| Error::Process("pty child has no pid".into()))?;

        #[cfg(unix)]
        unsafe {
            libc::setpgid(pid as i32, pid as i32);
        }

        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| Error::Process(error.to_string()))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|error| Error::Process(error.to_string()))?;
        let _ = writer.flush();

        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path(id))
            .map_err(|error| Error::Process(error.to_string()))?;

        let shared = Arc::new(SessionShared::new(log, self.activity.clone()));

        let reader_shared = shared.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                reader_shared.push_bytes(&buf[..n]);
            }
        });

        self.sessions.lock().unwrap().insert(
            id.to_string(),
            RunningApp {
                pid,
                shared: shared.clone(),
                pty: Mutex::new(Some(Pty {
                    writer,
                    _master: pair.master,
                })),
            },
        );

        let wait_shared = shared;
        let wait_sessions = self.sessions.clone();
        thread::spawn(move || {
            let mut child = child;
            let code = match child.wait() {
                Ok(status) => i32::try_from(status.exit_code()).unwrap_or(1),
                Err(_) => 1,
            };
            *wait_shared.exited.lock().unwrap() = Some(code);
            wait_shared.activity.notify_waiters();
            // The record stays for its exit code; the descriptors needn't. `pty` can already
            // belong to a newer session under this id, hence the check.
            if let Some(app) = wait_sessions
                .lock()
                .unwrap()
                .values()
                .find(|app| Arc::ptr_eq(&app.shared, &wait_shared))
            {
                app.pty.lock().unwrap().take();
            }
        });
        Ok(pid)
    }

    #[cfg(test)]
    pub fn output(&self) -> String {
        self.output_session("default")
    }

    pub fn output_session(&self, id: &str) -> String {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|app| {
                app.shared
                    .lines
                    .lock()
                    .unwrap()
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    pub fn drain_pending(&self, id: &str) -> Vec<String> {
        let from_session = self
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|app| std::mem::take(&mut *app.shared.pending.lock().unwrap()))
            .unwrap_or_default();
        let from_log = self
            .logs
            .lock()
            .unwrap()
            .get(id)
            .map(|shared| std::mem::take(&mut *shared.pending.lock().unwrap()))
            .unwrap_or_default();
        let mut merged = from_session;
        merged.extend(from_log);
        merged
    }

    /// The buffered lines of a workspace log, marking them all as delivered.
    pub fn take_log_snapshot(&self, id: &str) -> Vec<String> {
        let logs = self.logs.lock().unwrap();
        let Some(shared) = logs.get(id) else {
            return Vec::new();
        };
        let lines = shared.lines.lock().unwrap();
        shared.pending.lock().unwrap().clear();
        lines.iter().cloned().collect()
    }

    #[cfg(test)]
    pub fn write_input(&self, id: &str, data: &[u8]) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let app = sessions
            .get(id)
            .ok_or_else(|| Error::Process(format!("no session {id}")))?;
        let mut pty = app.pty.lock().unwrap();
        let pty = pty
            .as_mut()
            .ok_or_else(|| Error::Process(format!("session {id} has exited")))?;
        pty.writer
            .write_all(data)
            .map_err(|error| Error::Process(error.to_string()))?;
        Ok(())
    }

    #[cfg(test)]
    pub fn detected_port(&self) -> Option<u16> {
        detect_port(&self.output())
    }

    #[cfg(test)]
    pub fn current_pid(&self) -> Option<u32> {
        self.sessions
            .lock()
            .unwrap()
            .get("default")
            .map(|app| app.pid)
    }

    pub fn exit_code(&self, id: &str) -> Option<i32> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .and_then(|app| *app.shared.exited.lock().unwrap())
    }

    pub fn was_killed(&self, id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|app| app.shared.killed.load(Ordering::SeqCst))
            .unwrap_or(false)
    }

    pub fn clear_log(&self, id: &str) {
        let clear = |shared: &SessionShared| {
            shared.lines.lock().unwrap().clear();
            shared.pending.lock().unwrap().clear();
            shared.fragment.lock().unwrap().clear();
        };
        if let Some(shared) = self.logs.lock().unwrap().get(id) {
            clear(shared);
        }
        if let Some(app) = self.sessions.lock().unwrap().get(id) {
            clear(&app.shared);
        }
    }

    pub fn pids(&self) -> Vec<(String, u32)> {
        self.sessions
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, app)| app.shared.exited.lock().unwrap().is_none())
            .map(|(id, app)| (id.clone(), app.pid))
            .collect()
    }

    #[cfg(test)]
    pub fn stop(&self) -> Result<()> {
        self.stop_session("default")
    }

    pub fn stop_session(&self, id: &str) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let Some(app) = sessions.get(id) else {
            return Ok(());
        };
        app.shared.killed.store(true, Ordering::SeqCst);
        kill_group(app.pid, libc::SIGTERM)
    }

    pub fn kill_session(&self, id: &str) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let Some(app) = sessions.get(id) else {
            return Ok(());
        };
        kill_group(app.pid, libc::SIGKILL)
    }

    #[cfg(test)]
    pub fn wait_exit(&self, timeout: Duration) -> Result<i32> {
        self.wait_exit_session("default", timeout)
    }

    #[cfg(test)]
    pub fn wait_exit_session(&self, id: &str, timeout: Duration) -> Result<i32> {
        let started = std::time::Instant::now();
        loop {
            if let Some(code) = self.exit_code(id) {
                return Ok(code);
            }
            if started.elapsed() > timeout {
                return Err(Error::Process("timed out waiting for exit".into()));
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    /// For quitting, where there's no runtime left to wait on.
    pub fn stop_all(&self) -> Result<()> {
        let ids: Vec<String> = self.sessions.lock().unwrap().keys().cloned().collect();
        for id in &ids {
            let _ = self.stop_session(id);
        }
        let started = std::time::Instant::now();
        while started.elapsed() < STOP_GRACE && !self.all_exited(&ids) {
            thread::sleep(STOP_POLL);
        }
        self.kill_survivors(&ids);
        Ok(())
    }

    fn workspace_session_ids(&self, workspace_id: &str) -> Vec<String> {
        let prefix = format!("{workspace_id}-");
        self.sessions
            .lock()
            .unwrap()
            .keys()
            .filter(|id| id.as_str() == workspace_id || id.starts_with(&prefix))
            .cloned()
            .collect()
    }

    pub fn workspace_has_session(&self, workspace_id: &str) -> bool {
        !self.workspace_session_ids(workspace_id).is_empty()
    }

    pub async fn stop_workspace_sessions(&self, workspace_id: &str) -> Result<()> {
        let ids = self.workspace_session_ids(workspace_id);
        for id in &ids {
            let _ = self.stop_session(id);
        }
        let started = std::time::Instant::now();
        while started.elapsed() < STOP_GRACE && !self.all_exited(&ids) {
            tokio::time::sleep(STOP_POLL).await;
        }
        self.kill_survivors(&ids);
        Ok(())
    }

    fn all_exited(&self, ids: &[String]) -> bool {
        ids.iter().all(|id| self.exit_code(id).is_some())
    }

    fn kill_survivors(&self, ids: &[String]) {
        for id in ids {
            if self.exit_code(id).is_none() {
                let _ = self.kill_session(id);
            }
        }
    }

    /// Drop everything kept for a workspace that's gone: sessions, buffered logs, log files.
    pub fn forget_workspace(&self, workspace_id: &str) {
        let prefix = format!("{workspace_id}-");
        let belongs = |id: &str| id == workspace_id || id.starts_with(&prefix);
        let mut ids: Vec<String> = Vec::new();
        self.sessions.lock().unwrap().retain(|id, _| {
            let keep = !belongs(id);
            if !keep {
                ids.push(id.clone());
            }
            keep
        });
        self.logs.lock().unwrap().retain(|id, _| {
            let keep = !belongs(id);
            if !keep {
                ids.push(id.clone());
            }
            keep
        });
        for id in ids {
            let _ = std::fs::remove_file(log_path(&id));
        }
    }
}

impl SessionShared {
    fn new(log: File, activity: Arc<Notify>) -> Self {
        Self {
            lines: Mutex::new(VecDeque::new()),
            pending: Mutex::new(Vec::new()),
            fragment: Mutex::new(String::new()),
            log: Mutex::new(log),
            log_bytes: AtomicU64::new(0),
            exited: Mutex::new(None),
            killed: AtomicBool::new(false),
            activity,
        }
    }

    fn push_bytes(&self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        if let Ok(mut log) = self.log.lock() {
            let written = self
                .log_bytes
                .fetch_add(text.len() as u64, Ordering::Relaxed);
            if written >= LOG_FILE_CAP {
                let _ = log.set_len(0);
                self.log_bytes.store(0, Ordering::Relaxed);
            }
            let _ = log.write_all(text.as_bytes());
        }
        let mut fragment = self.fragment.lock().unwrap();
        fragment.push_str(&text);
        while let Some(index) = fragment.find('\n') {
            let mut line = fragment.drain(..=index).collect::<String>();
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            self.push_line(line);
        }
        keep_tail(&mut fragment, FRAGMENT_CAP);
    }

    fn push_line(&self, line: String) {
        {
            let mut lines = self.lines.lock().unwrap();
            // Dev servers clear the screen by printing a screenful of newlines.
            if is_blank(&line) && lines.back().is_some_and(|last| is_blank(last)) {
                return;
            }
            if lines.len() == RING_CAP {
                lines.pop_front();
            }
            lines.push_back(line.clone());
        }
        {
            // Nobody drains this until the Output tab opens, and the ring is all it replays.
            let mut pending = self.pending.lock().unwrap();
            if pending.len() >= RING_CAP * 2 {
                pending.drain(..RING_CAP);
            }
            pending.push(line);
        }
        self.activity.notify_waiters();
    }
}

fn keep_tail(text: &mut String, cap: usize) {
    if text.len() <= cap {
        return;
    }
    let mut start = text.len() - cap;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text.drain(..start);
}

pub fn detect_port(output: &str) -> Option<u16> {
    let output = strip_ansi(output);
    for marker in ["localhost:", "127.0.0.1:", "0.0.0.0:"] {
        for (index, _) in output.match_indices(marker) {
            let digits: String = output[index + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(port) = digits.parse() {
                return Some(port);
            }
        }
    }
    None
}

/// Whether the server announced `port` with an `https://` URL, so links to it should too.
pub fn serves_https(output: &str, port: u16) -> bool {
    let output = strip_ansi(output);
    ["localhost", "127.0.0.1", "0.0.0.0"]
        .iter()
        .any(|host| output.contains(&format!("https://{host}:{port}")))
}

/// Removes ANSI escape sequences (CSI like `\x1b[1m`, and OSC like hyperlinks).
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn is_blank(line: &str) -> bool {
    strip_ansi(line).trim().is_empty()
}

fn kill_group(pid: u32, signal: i32) -> Result<()> {
    let pgid = pid as i32;
    let result = unsafe { libc::killpg(pgid, signal) };
    if result != 0 {
        unsafe {
            libc::kill(pgid, signal);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn python_script(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/process")
            .join(name)
    }

    fn supervisor() -> ProcessSupervisor {
        ProcessSupervisor::new(Arc::new(RwLock::new(ShellEnv::new())))
    }

    #[test]
    fn spots_an_https_banner() {
        assert!(serves_https(
            "  \x1b[32m➜\x1b[39m  Local:   \x1b[36mhttps://localhost:\x1b[1m5173\x1b[22m/",
            5173
        ));
        assert!(!serves_https("Local: http://localhost:5173/", 5173));
        assert!(!serves_https(
            "https://localhost:4000/ and localhost:5173",
            5173
        ));
    }

    #[test]
    fn reads_port_from_dev_server_banner() {
        assert_eq!(detect_port("Local:   http://127.0.0.1:5173/\n"), Some(5173));
        assert_eq!(detect_port("ready on localhost:3000"), Some(3000));
        assert_eq!(
            detect_port(
                "  \x1b[32m➜\x1b[39m  \x1b[1mLocal\x1b[22m:   \x1b[36mhttp://localhost:\x1b[1m5173\x1b[22m/\x1b[39m"
            ),
            Some(5173)
        );
        assert_eq!(
            detect_port("see localhost: docs\nhttp://localhost:4000/"),
            Some(4000)
        );
    }

    #[test]
    fn caps_what_a_session_can_buffer() {
        let proc = supervisor();
        for line in 0..RING_CAP * 3 {
            proc.append_log_line("capped", line.to_string());
        }
        assert!(proc.drain_pending("capped").len() <= RING_CAP * 2);

        let mut fragment = "é".repeat(100);
        keep_tail(&mut fragment, 11);
        assert_eq!(fragment, "é".repeat(5));
    }

    #[test]
    fn exited_sessions_leave_metrics_and_forgetting_clears_them() {
        let proc = supervisor();
        proc.spawn_session("ws-setup-1", "true", &[], None, &[])
            .unwrap();
        proc.append_log_line("ws", "hello");
        proc.wait_exit_session("ws-setup-1", Duration::from_secs(5))
            .unwrap();
        let started = std::time::Instant::now();
        while !proc.pids().is_empty() && started.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(proc.pids().is_empty());
        // Monitors still read the exit code after the process is gone.
        assert_eq!(proc.exit_code("ws-setup-1"), Some(0));
        assert!(proc.workspace_has_session("ws"));

        proc.forget_workspace("ws");
        assert!(!proc.workspace_has_session("ws"));
        assert!(proc.take_log_snapshot("ws").is_empty());
        assert!(!log_path("ws").exists());
        assert!(!log_path("ws-setup-1").exists());
    }

    #[test]
    fn collapses_runs_of_blank_lines() {
        let proc = supervisor();
        for line in ["a", "", "", "\x1b[1;1H\x1b[0J", "b"] {
            proc.append_log_line("blank-runs", line);
        }
        assert_eq!(proc.take_log_snapshot("blank-runs"), vec!["a", "", "b"]);
        assert!(proc.drain_pending("blank-runs").is_empty());
    }

    #[test]
    fn pty_detects_port_stop_kills_tree_crash_reports_code() {
        let first = supervisor();
        let script = python_script("fake_dev_server.py");
        first.spawn("python3", &[script.to_str().unwrap()]).unwrap();

        let started = std::time::Instant::now();
        let port = loop {
            if let Some(port) = first.detected_port() {
                break port;
            }
            if started.elapsed() > Duration::from_secs(5) {
                panic!("no port in output: {}", first.output());
            }
            thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(port, 8765);

        let pid = first.current_pid().unwrap();
        first.stop().unwrap();
        thread::sleep(Duration::from_millis(400));
        let still_alive = unsafe { libc::kill(pid as i32, 0) } == 0;
        assert!(!still_alive, "process group should be dead after Stop");

        let crashing = supervisor();
        let crash = python_script("fake_crash.py");
        crashing
            .spawn("python3", &[crash.to_str().unwrap()])
            .unwrap();
        let code = crashing.wait_exit(Duration::from_secs(5)).unwrap();
        assert_eq!(code, 17);
    }

    #[test]
    fn ring_buffer_keeps_500_lines_and_accepts_input() {
        let proc = supervisor();
        proc.spawn(
            "python3",
            &[
                "-c",
                "print('hello', flush=True); import time; time.sleep(2)",
            ],
        )
        .unwrap();
        thread::sleep(Duration::from_millis(400));
        let pending = proc.drain_pending("default");
        assert!(pending.iter().any(|line| line.contains("hello")));
        let _ = proc.write_input("default", b"x");
        proc.stop().unwrap();
    }
}
