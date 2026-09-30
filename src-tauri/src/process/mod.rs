use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;

const RING_CAP: usize = 500;

/// Setup and run commands in a PTY, each in its own process group.
#[derive(Clone)]
pub struct ProcessSupervisor {
    env: Arc<RwLock<ShellEnv>>,
    sessions: Arc<Mutex<HashMap<String, RunningApp>>>,
    logs: Arc<Mutex<HashMap<String, Arc<SessionShared>>>>,
}

struct SessionShared {
    lines: Mutex<VecDeque<String>>,
    pending: Mutex<Vec<String>>,
    fragment: Mutex<String>,
    log: Mutex<File>,
    exited: Mutex<Option<i32>>,
    killed: AtomicBool,
}

struct RunningApp {
    pid: u32,
    shared: Arc<SessionShared>,
    writer: Mutex<Box<dyn Write + Send>>,
    _master: Box<dyn MasterPty + Send>,
}

impl ProcessSupervisor {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self {
            env,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn append_log_line(&self, id: &str, line: impl Into<String>) {
        let line = line.into();
        let shared = {
            let mut logs = self.logs.lock().unwrap();
            logs.entry(id.to_string())
                .or_insert_with(|| {
                    Arc::new(SessionShared {
                        lines: Mutex::new(VecDeque::new()),
                        pending: Mutex::new(Vec::new()),
                        fragment: Mutex::new(String::new()),
                        log: Mutex::new(
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(std::env::temp_dir().join(format!("cormux-pty-{id}.log")))
                                .unwrap_or_else(|_| {
                                    OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(std::env::temp_dir().join("cormux-pty-fallback.log"))
                                        .expect("fallback pty log")
                                }),
                        ),
                        exited: Mutex::new(None),
                        killed: AtomicBool::new(false),
                    })
                })
                .clone()
        };
        shared.push_line(line);
    }

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

        let log_path = std::env::temp_dir().join(format!("cormux-pty-{id}.log"));
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|error| Error::Process(error.to_string()))?;

        let shared = Arc::new(SessionShared {
            lines: Mutex::new(VecDeque::new()),
            pending: Mutex::new(Vec::new()),
            fragment: Mutex::new(String::new()),
            log: Mutex::new(log),
            exited: Mutex::new(None),
            killed: AtomicBool::new(false),
        });

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

        let wait_shared = shared.clone();
        thread::spawn(move || {
            let mut child = child;
            let code = match child.wait() {
                Ok(status) => i32::try_from(status.exit_code()).unwrap_or(1),
                Err(_) => 1,
            };
            *wait_shared.exited.lock().unwrap() = Some(code);
        });

        self.sessions.lock().unwrap().insert(
            id.to_string(),
            RunningApp {
                pid,
                shared,
                writer: Mutex::new(writer),
                _master: pair.master,
            },
        );
        Ok(pid)
    }

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

    pub fn write_input(&self, id: &str, data: &[u8]) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let app = sessions
            .get(id)
            .ok_or_else(|| Error::Process(format!("no session {id}")))?;
        app.writer
            .lock()
            .unwrap()
            .write_all(data)
            .map_err(|error| Error::Process(error.to_string()))?;
        Ok(())
    }

    pub fn detected_port(&self) -> Option<u16> {
        detect_port(&self.output())
    }

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
            .map(|(id, app)| (id.clone(), app.pid))
            .collect()
    }

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

    pub fn wait_exit(&self, timeout: Duration) -> Result<i32> {
        self.wait_exit_session("default", timeout)
    }

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

    pub fn stop_all(&self) -> Result<()> {
        let ids: Vec<String> = self.sessions.lock().unwrap().keys().cloned().collect();
        self.stop_sessions(ids)
    }

    pub fn workspace_has_session(&self, workspace_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .keys()
            .any(|id| id.as_str() == workspace_id || id.starts_with(&format!("{workspace_id}-")))
    }

    pub fn stop_workspace_sessions(&self, workspace_id: &str) -> Result<()> {
        let ids: Vec<String> = self
            .sessions
            .lock()
            .unwrap()
            .keys()
            .filter(|id| id.as_str() == workspace_id || id.starts_with(&format!("{workspace_id}-")))
            .cloned()
            .collect();
        self.stop_sessions(ids)
    }

    fn stop_sessions(&self, ids: Vec<String>) -> Result<()> {
        for id in &ids {
            let _ = self.stop_session(id);
        }
        thread::sleep(Duration::from_millis(200));
        for id in ids {
            if self.exit_code(&id).is_none() {
                let _ = self.kill_session(&id);
            }
        }
        Ok(())
    }
}

impl SessionShared {
    fn push_bytes(&self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        if let Ok(mut log) = self.log.lock() {
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
        self.pending.lock().unwrap().push(line);
    }
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
    fn collapses_runs_of_blank_lines() {
        let proc = supervisor();
        for line in ["a", "", "", "\x1b[1;1H\x1b[0J", "b"] {
            proc.append_log_line("ws", line);
        }
        assert_eq!(proc.take_log_snapshot("ws"), vec!["a", "", "b"]);
        assert!(proc.drain_pending("ws").is_empty());
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
