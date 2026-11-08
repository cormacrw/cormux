//! Running shell commands in PTYs: worktree setup, apps, and repo setup tests. Each session
//! belongs to a `SessionId`; each workspace also has a log, the lines its Output tab shows,
//! that sessions spawned for it forward into.

mod session;

use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::{Notify, RwLock, watch};

use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;

pub use session::{HarnessEnv, SessionId, ShellCommand};

const RING_CAP: usize = 500;
/// A line that never ends (`\r`-only progress bars) keeps only its tail.
const FRAGMENT_CAP: usize = 64 * 1024;
/// A log past this starts over; nothing reads it back, it's there for debugging.
const LOG_FILE_CAP: u64 = 10 * 1024 * 1024;
/// How long a stopped session gets to exit before its group is sent SIGKILL.
const SIGKILL_AFTER: Duration = Duration::from_millis(200);
const STOP_POLL: Duration = Duration::from_millis(25);
/// After its process exits, how long a session waits for the rest of its output. A background
/// child still holding the terminal keeps it open, so this can't wait for EOF forever.
const OUTPUT_DRAIN: Duration = Duration::from_secs(1);
const LOG_FILE_PREFIX: &str = "cormux-pty-";

#[derive(Clone)]
pub struct ProcessSupervisor {
    env: Arc<RwLock<ShellEnv>>,
    sessions: Arc<Mutex<HashMap<SessionId, RunningApp>>>,
    logs: Arc<Mutex<HashMap<String, Arc<Lines>>>>,
    /// Woken on every new line of any session or log.
    activity: Arc<Notify>,
}

/// One line of a workspace's log: what a command printed, or what Cormux says about the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct LogEntry {
    pub kind: LogKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LogKind {
    /// What a command printed.
    Output,
    /// A command Cormux started.
    Command,
    /// A point in the run: setup done, the app restarted, stopped or crashed.
    Marker,
    /// Cormux interrupted the app.
    Interrupt,
    /// Cormux telling you something about the run.
    Note,
    /// Cormux telling you something went differently than asked, like a port being taken.
    Warning,
}

impl LogEntry {
    pub fn new(kind: LogKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }

    fn is_blank_output(&self) -> bool {
        self.kind == LogKind::Output && is_blank(&self.text)
    }
}

/// A session's progress: lines printed so far, and its exit code once it has exited and its
/// output has been read to the end.
#[derive(Debug, Clone, Copy, Default)]
struct SessionState {
    lines: u64,
    exit: Option<i32>,
}

/// A buffer of output lines: a session's, or a workspace log.
struct Lines {
    ring: Mutex<VecDeque<LogEntry>>,
    /// Lines ever pushed into `ring`, so a reader's cursor still means the same line after the
    /// ring drops its oldest. Only changed with `ring` locked.
    pushed: AtomicU64,
    fragment: Mutex<String>,
    /// The tail of the last read that was only part of a character; the next read finishes it.
    partial: Mutex<Vec<u8>>,
    log: Mutex<File>,
    log_bytes: AtomicU64,
    /// The workspace log that gets a copy of every line.
    forward: Option<Arc<Lines>>,
    state: watch::Sender<SessionState>,
    activity: Arc<Notify>,
}

struct RunningApp {
    pid: u32,
    lines: Arc<Lines>,
    /// The process has been waited for, so its pid may already belong to another process.
    reaped: Arc<AtomicBool>,
    killed: AtomicBool,
    /// The PTY's descriptors, released as soon as the process exits.
    pty: Mutex<Option<Pty>>,
}

struct Pty {
    /// Held so the app's stdin stays open; dropping it would send EOF.
    _writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
}

pub struct SessionEvents {
    state: watch::Receiver<SessionState>,
}

impl SessionEvents {
    /// The exit code, once the process has exited and its output has been read.
    pub fn exit_code(&self) -> Option<i32> {
        self.state.borrow().exit
    }

    pub async fn changed(&mut self) {
        if self.state.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }

    pub async fn exited(&mut self) -> i32 {
        match self.state.wait_for(|state| state.exit.is_some()).await {
            Ok(state) => state.exit.unwrap_or(-1),
            Err(_) => -1,
        }
    }
}

/// How `run_to_exit` ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Code(i32),
    /// Ran past its time and was stopped.
    TimedOut,
}

fn log_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{LOG_FILE_PREFIX}{name}.log"))
}

fn open_log(name: &str) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path(name))
        .map_err(|error| Error::Process(format!("opening the log for {name}: {error}")))
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
            if name.starts_with(LOG_FILE_PREFIX)
                && name.ends_with(".log")
                && let Err(error) = std::fs::remove_file(entry.path())
            {
                log::debug!("removing stale log {name}: {error}");
            }
        }
    }

    fn workspace_log(&self, workspace_id: &str) -> Arc<Lines> {
        let mut logs = self.logs.lock().unwrap();
        if let Some(lines) = logs.get(workspace_id) {
            return lines.clone();
        }
        let file = open_log(workspace_id).unwrap_or_else(|error| {
            log::warn!("{error}; writing it nowhere");
            File::create("/dev/null").expect("/dev/null opens")
        });
        let lines = Arc::new(Lines::new(file, None, self.activity.clone()));
        logs.insert(workspace_id.to_string(), lines.clone());
        lines
    }

    /// Adds Cormux's own line to a workspace's log.
    pub fn log(&self, workspace_id: &str, kind: LogKind, text: impl Into<String>) {
        self.workspace_log(workspace_id)
            .push_entry(LogEntry::new(kind, text));
    }

    /// Runs `command` in the user's login shell until it exits, or until `timeout` passes and
    /// it's stopped. Its output also goes to `forward_to`'s workspace log.
    pub async fn run_to_exit(
        &self,
        id: SessionId,
        command: &ShellCommand,
        timeout: Duration,
        forward_to: Option<&str>,
    ) -> Result<Exit> {
        let mut events = self.spawn_shell(id.clone(), command, forward_to).await?;
        let exit = match tokio::time::timeout(timeout, events.exited()).await {
            Ok(code) => Exit::Code(code),
            Err(_) => Exit::TimedOut,
        };
        // Whatever it left running in its group goes with it.
        if let Err(error) = self.stop_session(&id) {
            log::warn!("stopping {id}: {error}");
        }
        Ok(exit)
    }

    /// Starts `command` in the user's login shell, replacing any session under `id`.
    pub async fn spawn_shell(
        &self,
        id: SessionId,
        command: &ShellCommand,
        forward_to: Option<&str>,
    ) -> Result<SessionEvents> {
        let env = self.env.read().await.clone();
        let mut builder = CommandBuilder::new(env.shell());
        builder.args(["-l", "-c", command.cmd.as_str()]);
        builder.cwd(&command.cwd);
        env.apply(&mut builder);
        for (key, value) in &command.env {
            builder.env(key, value);
        }
        self.spawn(id, builder, forward_to)
    }

    fn spawn(
        &self,
        id: SessionId,
        command: CommandBuilder,
        forward_to: Option<&str>,
    ) -> Result<SessionEvents> {
        if let Err(error) = self.stop_session(&id) {
            log::warn!("stopping the previous {id}: {error}");
        }

        let pair = NativePtySystem::default()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| Error::Process(error.to_string()))?;

        let child = pair
            .slave
            .spawn_command(command)
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
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| Error::Process(error.to_string()))?;

        let forward = forward_to.map(|workspace_id| self.workspace_log(workspace_id));
        let lines = Arc::new(Lines::new(
            open_log(&id.to_string())?,
            forward,
            self.activity.clone(),
        ));
        let events = SessionEvents {
            state: lines.state.subscribe(),
        };

        let (eof_tx, eof_rx) = mpsc::channel::<()>();
        let reader_lines = lines.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                reader_lines.push_bytes(&buf[..n]);
            }
            reader_lines.finish_bytes();
            drop(eof_tx);
        });

        let reaped = Arc::new(AtomicBool::new(false));
        self.sessions.lock().unwrap().insert(
            id,
            RunningApp {
                pid,
                lines: lines.clone(),
                reaped: reaped.clone(),
                killed: AtomicBool::new(false),
                pty: Mutex::new(Some(Pty {
                    _writer: writer,
                    _master: pair.master,
                })),
            },
        );

        let sessions = self.sessions.clone();
        thread::spawn(move || {
            let mut child = child;
            let code = match child.wait() {
                Ok(status) => i32::try_from(status.exit_code()).unwrap_or(1),
                Err(_) => 1,
            };
            reaped.store(true, Ordering::SeqCst);
            // The last lines can still be in the terminal; whoever waits on the exit wants them.
            let _ = eof_rx.recv_timeout(OUTPUT_DRAIN);
            lines.state.send_modify(|state| state.exit = Some(code));
            lines.activity.notify_waiters();
            // The record stays for its exit code; the descriptors needn't. `pty` can already
            // belong to a newer session under this id, hence the check.
            if let Some(app) = sessions
                .lock()
                .unwrap()
                .values()
                .find(|app| Arc::ptr_eq(&app.lines, &lines))
            {
                app.pty.lock().unwrap().take();
            }
        });
        Ok(events)
    }

    pub fn output(&self, id: &SessionId) -> String {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|app| {
                let ring = app.lines.ring.lock().unwrap();
                ring.iter()
                    .map(|entry| entry.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    /// A workspace log's lines after `cursor`, which moves past them; start it at 0 for every
    /// buffered line. Each reader keeps its own cursor, so two readers both get every line. One
    /// that fell further behind than the ring holds resumes at the ring's oldest line.
    pub fn log_since(&self, workspace_id: &str, cursor: &mut u64) -> Vec<LogEntry> {
        let Some(lines) = self.logs.lock().unwrap().get(workspace_id).cloned() else {
            return Vec::new();
        };
        let ring = lines.ring.lock().unwrap();
        let end = lines.pushed.load(Ordering::Relaxed);
        let start = end - ring.len() as u64;
        // Past the end means the log was forgotten and started again.
        if *cursor < start || *cursor > end {
            *cursor = start;
        }
        let skip = usize::try_from(*cursor - start).unwrap_or(usize::MAX);
        *cursor = end;
        ring.iter().skip(skip).cloned().collect()
    }

    #[cfg(test)]
    pub fn log_snapshot(&self, workspace_id: &str) -> Vec<LogEntry> {
        self.log_since(workspace_id, &mut 0)
    }

    #[cfg(test)]
    pub fn log_texts(&self, workspace_id: &str) -> Vec<String> {
        self.log_snapshot(workspace_id)
            .into_iter()
            .map(|entry| entry.text)
            .collect()
    }

    pub fn was_killed(&self, id: &SessionId) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|app| app.killed.load(Ordering::SeqCst))
    }

    pub fn clear_log(&self, workspace_id: &str) {
        if let Some(lines) = self.logs.lock().unwrap().get(workspace_id) {
            lines.ring.lock().unwrap().clear();
            lines.fragment.lock().unwrap().clear();
        }
    }

    /// Sessions whose process hasn't exited.
    #[cfg(test)]
    pub fn pids(&self) -> Vec<(SessionId, u32)> {
        self.sessions
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, app)| !app.reaped.load(Ordering::SeqCst))
            .map(|(id, app)| (id.clone(), app.pid))
            .collect()
    }

    /// Asks the session's process group to end (SIGTERM).
    pub fn stop_session(&self, id: &SessionId) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let Some(app) = sessions.get(id) else {
            return Ok(());
        };
        app.killed.store(true, Ordering::SeqCst);
        kill_group(app.pid, libc::SIGTERM, app.reaped.load(Ordering::SeqCst))
    }

    fn kill_session(&self, id: &SessionId) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let Some(app) = sessions.get(id) else {
            return Ok(());
        };
        kill_group(app.pid, libc::SIGKILL, app.reaped.load(Ordering::SeqCst))
    }

    /// For quitting, where there's no runtime left to wait on.
    pub fn stop_all(&self) {
        let ids: Vec<SessionId> = self.sessions.lock().unwrap().keys().cloned().collect();
        self.signal(&ids);
        let started = std::time::Instant::now();
        while started.elapsed() < SIGKILL_AFTER && !self.all_reaped(&ids) {
            thread::sleep(STOP_POLL);
        }
        self.kill_survivors(&ids);
    }

    fn workspace_session_ids(&self, workspace_id: &str) -> Vec<SessionId> {
        self.sessions
            .lock()
            .unwrap()
            .keys()
            .filter(|id| id.owner_workspace() == Some(workspace_id))
            .cloned()
            .collect()
    }

    /// Stops every session the workspace owns, killing what's left after a grace period.
    pub async fn stop_workspace_sessions(&self, workspace_id: &str) {
        let ids = self.workspace_session_ids(workspace_id);
        self.signal(&ids);
        let exits: Vec<_> = {
            let sessions = self.sessions.lock().unwrap();
            ids.iter()
                .filter_map(|id| sessions.get(id))
                .map(|app| app.lines.state.subscribe())
                .collect()
        };
        let all_exited = async {
            for mut state in exits {
                let _ = state.wait_for(|state| state.exit.is_some()).await;
            }
        };
        let _ = tokio::time::timeout(SIGKILL_AFTER, all_exited).await;
        self.kill_survivors(&ids);
    }

    fn signal(&self, ids: &[SessionId]) {
        for id in ids {
            if let Err(error) = self.stop_session(id) {
                log::warn!("stopping {id}: {error}");
            }
        }
    }

    fn all_reaped(&self, ids: &[SessionId]) -> bool {
        let sessions = self.sessions.lock().unwrap();
        ids.iter().all(|id| {
            sessions
                .get(id)
                .is_none_or(|app| app.reaped.load(Ordering::SeqCst))
        })
    }

    fn kill_survivors(&self, ids: &[SessionId]) {
        for id in ids {
            let alive = self
                .sessions
                .lock()
                .unwrap()
                .get(id)
                .is_some_and(|app| !app.reaped.load(Ordering::SeqCst));
            if alive && let Err(error) = self.kill_session(id) {
                log::warn!("killing {id}: {error}");
            }
        }
    }

    /// Drop everything kept for a workspace that's gone: sessions, its log, log files.
    pub fn forget_workspace(&self, workspace_id: &str) {
        let mut files = vec![log_path(workspace_id)];
        self.sessions.lock().unwrap().retain(|id, _| {
            let keep = id.owner_workspace() != Some(workspace_id);
            if !keep {
                files.push(log_path(&id.to_string()));
            }
            keep
        });
        self.logs.lock().unwrap().remove(workspace_id);
        for file in files {
            if let Err(error) = std::fs::remove_file(&file)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                log::debug!("removing {}: {error}", file.display());
            }
        }
    }
}

impl Lines {
    fn new(log: File, forward: Option<Arc<Lines>>, activity: Arc<Notify>) -> Self {
        Self {
            ring: Mutex::new(VecDeque::new()),
            pushed: AtomicU64::new(0),
            fragment: Mutex::new(String::new()),
            partial: Mutex::new(Vec::new()),
            log: Mutex::new(log),
            log_bytes: AtomicU64::new(0),
            forward,
            state: watch::Sender::new(SessionState::default()),
            activity,
        }
    }

    /// Reads land in 4 KiB pieces that can split a multi-byte character (an arrow or an accent
    /// in a build banner), so the bytes of an unfinished one wait for the next read instead of
    /// turning into replacement characters.
    fn push_bytes(&self, bytes: &[u8]) {
        let mut partial = self.partial.lock().unwrap();
        let mut buffer = std::mem::take(&mut *partial);
        buffer.extend_from_slice(bytes);
        let complete = complete_utf8_len(&buffer);
        partial.extend_from_slice(&buffer[complete..]);
        drop(partial);
        buffer.truncate(complete);
        self.push_text(&String::from_utf8_lossy(&buffer));
    }

    /// Whatever a closed stream left unfinished still shows, as best it can.
    fn finish_bytes(&self) {
        let rest = std::mem::take(&mut *self.partial.lock().unwrap());
        if !rest.is_empty() {
            self.push_text(&String::from_utf8_lossy(&rest));
        }
    }

    fn push_text(&self, text: &str) {
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
        fragment.push_str(text);
        while let Some(index) = fragment.find('\n') {
            let mut line = fragment.drain(..=index).collect::<String>();
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            self.push_entry(LogEntry::new(LogKind::Output, line));
        }
        keep_tail(&mut fragment, FRAGMENT_CAP);
    }

    fn push_entry(&self, line: LogEntry) {
        {
            let mut ring = self.ring.lock().unwrap();
            // Dev servers clear the screen by printing a screenful of newlines.
            if line.is_blank_output() && ring.back().is_some_and(LogEntry::is_blank_output) {
                return;
            }
            if ring.len() == RING_CAP {
                ring.pop_front();
            }
            ring.push_back(line.clone());
            self.pushed.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(forward) = &self.forward {
            forward.push_entry(line);
        }
        self.state.send_modify(|state| state.lines += 1);
        self.activity.notify_waiters();
    }
}

/// How much of `bytes` can be decoded now: everything up to a trailing character that is
/// still missing its last bytes. Only the tail is looked at, so invalid bytes earlier on are
/// replaced by the lossy decode rather than holding everything after them back.
fn complete_utf8_len(bytes: &[u8]) -> usize {
    for back in 1..=bytes.len().min(3) {
        let index = bytes.len() - back;
        let byte = bytes[index];
        if byte & 0xC0 == 0x80 {
            continue;
        }
        let needed = match byte {
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            // ASCII, or a byte that can't start a character and will be replaced.
            _ => 1,
        };
        return if needed > back { index } else { bytes.len() };
    }
    bytes.len()
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

/// Signals the session's process group, so its children go too.
///
/// Once the leader has been reaped (`reaped`), its pid is free for the OS to hand out again.
/// The group id stays reserved only while members of the group survive, so `killpg` is still
/// right for stragglers, but the plain-`kill` fallback would hit whatever process now has the
/// pid, and is skipped.
fn kill_group(pid: u32, signal: i32, reaped: bool) -> Result<()> {
    // pid 0 or one past i32::MAX would make killpg signal our own group, or every process.
    let Ok(pgid) = libc::pid_t::try_from(pid) else {
        return Ok(());
    };
    if pgid <= 0 {
        return Ok(());
    }
    // SAFETY: killpg takes plain integers and has no memory-safety preconditions.
    if unsafe { libc::killpg(pgid, signal) } == 0 || reaped {
        return Ok(());
    }
    // SAFETY: as above; the leader hasn't been reaped, so `pgid` is still our child's pid.
    if unsafe { libc::kill(pgid, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        // Already gone.
        return Ok(());
    }
    Err(Error::Process(format!("signalling process {pid}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn python_script(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/process")
            .join(name)
    }

    fn supervisor() -> ProcessSupervisor {
        ProcessSupervisor::new(Arc::new(RwLock::new(ShellEnv::new())))
    }

    fn shell(cmd: &str) -> ShellCommand {
        ShellCommand {
            cmd: cmd.into(),
            cwd: std::env::temp_dir(),
            env: Vec::new(),
        }
    }

    fn setup(workspace: &str, index: usize) -> SessionId {
        SessionId::Setup {
            workspace: workspace.into(),
            index,
        }
    }

    fn run(workspace: &str) -> SessionId {
        SessionId::Run {
            workspace: workspace.into(),
        }
    }

    fn python(proc: &ProcessSupervisor, id: SessionId, script: &Path) -> SessionEvents {
        let mut command = CommandBuilder::new("python3");
        command.arg(script);
        proc.spawn(id, command, None).unwrap()
    }

    fn pid_of(proc: &ProcessSupervisor, id: &SessionId) -> u32 {
        proc.sessions.lock().unwrap()[id].pid
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
    fn caps_what_a_log_can_buffer() {
        let proc = supervisor();
        for line in 0..RING_CAP * 3 {
            proc.log("capped", LogKind::Output, line.to_string());
        }
        assert_eq!(proc.log_snapshot("capped").len(), RING_CAP);
        let mut behind = 0;
        assert_eq!(proc.log_since("capped", &mut behind).len(), RING_CAP);
        assert_eq!(behind, (RING_CAP * 3) as u64);

        let mut fragment = "é".repeat(100);
        keep_tail(&mut fragment, 11);
        assert_eq!(fragment, "é".repeat(5));
    }

    #[tokio::test]
    async fn a_command_runs_to_its_exit_with_every_line_in_the_workspace_log() {
        let proc = supervisor();
        proc.log("ws", LogKind::Output, "cmd: count");
        let mut command =
            shell("for i in 1 2 3; do echo line $i; done; echo \"$HARNESS_BASE\"; exit 3");
        command.env.push(("HARNESS_BASE".into(), "develop".into()));

        let exit = proc
            .run_to_exit(
                setup("ws", 0),
                &command,
                Duration::from_secs(10),
                Some("ws"),
            )
            .await
            .unwrap();

        assert_eq!(exit, Exit::Code(3));
        assert_eq!(
            proc.log_texts("ws"),
            ["cmd: count", "line 1", "line 2", "line 3", "develop"]
        );
    }

    #[tokio::test]
    async fn a_command_waits_for_the_login_env_rather_than_running_without_it() {
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        let proc = ProcessSupervisor::new(env.clone());
        let loading = env.write().await;
        let running = tokio::spawn({
            let proc = proc.clone();
            async move {
                let command = shell(r#"test "$CORMUX_LOGIN_ENV" = loaded"#);
                proc.run_to_exit(setup("ws", 0), &command, Duration::from_secs(10), None)
                    .await
            }
        });
        tokio::task::yield_now().await;
        let mut loading = loading;
        *loading = ShellEnv::from_vars(
            std::env::vars().chain([("CORMUX_LOGIN_ENV".into(), "loaded".into())]),
        );
        drop(loading);
        assert_eq!(running.await.unwrap().unwrap(), Exit::Code(0));
    }

    #[tokio::test]
    async fn a_command_past_its_time_is_stopped() {
        let proc = supervisor();
        let exit = proc
            .run_to_exit(
                setup("ws", 0),
                &shell("sleep 30"),
                Duration::from_millis(300),
                None,
            )
            .await
            .unwrap();
        assert_eq!(exit, Exit::TimedOut);
        assert!(proc.was_killed(&setup("ws", 0)));
    }

    #[tokio::test]
    async fn exited_sessions_leave_their_code_and_forgetting_clears_them() {
        // Log files live in the shared temp folder, so this workspace's id is its own.
        let ws = format!("forget-{}", uuid::Uuid::new_v4());
        let proc = supervisor();
        let exit = proc
            .run_to_exit(
                setup(&ws, 1),
                &shell("true"),
                Duration::from_secs(10),
                Some(&ws),
            )
            .await
            .unwrap();
        assert_eq!(exit, Exit::Code(0));
        assert!(proc.pids().is_empty());
        assert!(!proc.workspace_session_ids(&ws).is_empty());
        // Another workspace's id can't claim it by sharing a prefix.
        assert!(proc.workspace_session_ids(&ws[..ws.len() - 1]).is_empty());

        proc.forget_workspace(&ws);
        assert!(proc.workspace_session_ids(&ws).is_empty());
        assert!(proc.log_snapshot(&ws).is_empty());
        assert!(!log_path(&ws).exists());
        assert!(!log_path(&setup(&ws, 1).to_string()).exists());
    }

    #[test]
    fn a_character_split_across_reads_stays_whole() {
        let proc = supervisor();
        proc.log("split", LogKind::Output, "warm up");
        let lines = proc.logs.lock().unwrap().get("split").unwrap().clone();
        let text = "➜  Local: ready\n".as_bytes();
        // The arrow is three bytes; cut it in the middle.
        lines.push_bytes(&text[..2]);
        lines.push_bytes(&text[2..]);
        assert_eq!(proc.log_texts("split"), vec!["warm up", "➜  Local: ready"]);
        // Invalid bytes are replaced rather than waited for; a cut-off tail still shows at EOF.
        assert_eq!(complete_utf8_len(b"ok\xff"), 3);
        assert_eq!(complete_utf8_len("é".as_bytes()), 2);
        assert_eq!(complete_utf8_len(&"é".as_bytes()[..1]), 0);
        assert_eq!(complete_utf8_len(&"a😀".as_bytes()[..4]), 1);
        assert_eq!(complete_utf8_len("a😀".as_bytes()), 5);
        assert_eq!(complete_utf8_len(b"\xffok\xe2\x9e"), 3);
        lines.push_bytes(&"end é".as_bytes()[..5]);
        lines.finish_bytes();
        lines.push_bytes(b"\n");
        assert_eq!(proc.log_texts("split").last().unwrap(), "end \u{fffd}");
    }

    #[test]
    fn collapses_runs_of_blank_lines() {
        let proc = supervisor();
        for line in ["a", "", "", "\x1b[1;1H\x1b[0J", "b"] {
            proc.log("blank-runs", LogKind::Output, line);
        }
        assert_eq!(proc.log_texts("blank-runs"), vec!["a", "", "b"]);
    }

    /// Bug: readers drained one shared buffer, so a second Output stream (one a webview reload
    /// left behind) took about half the lines from the live one.
    #[test]
    fn every_reader_of_a_log_gets_every_line() {
        let proc = supervisor();
        proc.log("shared", LogKind::Output, "before");
        let (mut first, mut second) = (0, 0);
        assert_eq!(proc.log_since("shared", &mut first).len(), 1);
        proc.log("shared", LogKind::Output, "one");
        let texts = |entries: Vec<LogEntry>| -> Vec<String> {
            entries.into_iter().map(|entry| entry.text).collect()
        };
        assert_eq!(
            texts(proc.log_since("shared", &mut second)),
            ["before", "one"]
        );
        proc.log("shared", LogKind::Output, "two");
        assert_eq!(texts(proc.log_since("shared", &mut first)), ["one", "two"]);
        assert_eq!(texts(proc.log_since("shared", &mut second)), ["two"]);
        assert!(proc.log_since("shared", &mut first).is_empty());

        proc.clear_log("shared");
        proc.log("shared", LogKind::Output, "after clearing");
        assert_eq!(
            texts(proc.log_since("shared", &mut first)),
            ["after clearing"]
        );

        proc.forget_workspace("shared");
        proc.log("shared", LogKind::Output, "started again");
        assert_eq!(
            texts(proc.log_since("shared", &mut second)),
            ["started again"]
        );
    }

    #[tokio::test]
    async fn stopping_a_workspace_kills_its_whole_tree() {
        let proc = supervisor();
        let id = run("ws");
        let mut events = python(&proc, id.clone(), &python_script("fake_dev_server.py"));
        while detect_port(&proc.output(&id)).is_none() {
            tokio::time::timeout(Duration::from_secs(5), events.changed())
                .await
                .expect("a port in the output");
        }
        assert_eq!(detect_port(&proc.output(&id)), Some(8765));

        let pid = pid_of(&proc, &id);
        proc.stop_workspace_sessions("ws").await;
        tokio::time::timeout(Duration::from_secs(5), events.exited())
            .await
            .unwrap();
        assert!(proc.was_killed(&id));
        let still_alive = unsafe { libc::kill(pid as i32, 0) } == 0;
        assert!(!still_alive, "process group should be dead after Stop");
    }

    #[tokio::test]
    async fn a_crash_reports_its_code_after_its_last_line() {
        let proc = supervisor();
        let id = run("ws");
        let mut events = python(&proc, id.clone(), &python_script("fake_crash.py"));
        let code = tokio::time::timeout(Duration::from_secs(5), events.exited())
            .await
            .unwrap();
        assert_eq!(code, 17);
        assert_eq!(proc.output(&id), "crashed");
        assert!(!proc.was_killed(&id));
    }
}
