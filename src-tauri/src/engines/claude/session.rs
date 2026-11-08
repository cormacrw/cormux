use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

#[cfg(test)]
use super::protocol::user_message;
use super::protocol::{
    Event, PermissionDecision, StdinMessage, decode_event, encode_line, initialize_request,
    interrupt_request, mcp_response, permission_response, set_effort_request, set_model_request,
    stop_task_request, user_prompt_message,
};
use crate::engines::claude_argv;
use crate::engines::prompt::Prompt;
use crate::error::{Error, Result};

/// How long the CLI gets to answer the initialize handshake. Once a session is up there is
/// no read timeout: Claude is silent between turns and while a long tool runs.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct SpawnOptions {
    pub binary: PathBuf,
    pub cwd: Option<PathBuf>,
    pub resume: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub extra_args: Vec<String>,
    /// When false, only `extra_args` are passed (used by the mock CLI fixture).
    pub use_default_args: bool,
    /// Give the session the Cormux MCP server, which `run_claude` answers.
    pub orchestrate: bool,
    /// Offer the experimental References panel's `attach_entity` tool, and say what it's for.
    pub references: bool,
    pub env: Option<HashMap<String, String>>,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("claude"),
            cwd: None,
            resume: None,
            model: None,
            effort: None,
            extra_args: Vec::new(),
            use_default_args: true,
            orchestrate: false,
            references: false,
            env: None,
        }
    }
}

/// One long-lived `claude -p` process speaking NDJSON on stdin/stdout.
const LINE_BUFFER_KEEP: usize = 1024 * 1024;

pub struct ClaudeSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// The line being read. It lives here, not in `next_event`, so a read cancelled by
    /// `select!` keeps what it had and the next call finishes the same line.
    pending: Vec<u8>,
    session_id: Option<String>,
    next_request_id: u64,
}

impl ClaudeSession {
    pub async fn spawn(options: SpawnOptions) -> Result<Self> {
        let mut args = if options.use_default_args {
            claude_argv(
                options.resume.as_deref(),
                options.model.as_deref(),
                options.effort.as_deref(),
                options.orchestrate,
                options.references,
            )
        } else {
            Vec::new()
        };
        args.extend(options.extra_args);

        let mut command = Command::new(&options.binary);
        command
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        if let Some(vars) = &options.env {
            command.env_clear();
            for (key, value) in vars {
                command.env(key, value);
            }
        }
        if options.use_default_args {
            command.env("CLAUDE_CODE_ENTRYPOINT", "sdk-ts");
        }
        if let Some(cwd) = &options.cwd {
            command.current_dir(cwd);
        }

        let mut child = command.spawn().map_err(|error| {
            Error::Engine(format!(
                "failed to spawn {}: {error}",
                options.binary.display()
            ))
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Engine("claude stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Engine("claude stdout missing".into()))?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                // Bytes, not `read_line`: one non-UTF-8 byte (from a hook or MCP server the
                // CLI passes through) would error and stop the drain, and a CLI writing to a
                // full, unread stderr pipe blocks.
                let mut line = Vec::new();
                loop {
                    line.clear();
                    match reader.read_until(b'\n', &mut line).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => log::warn!(
                            "claude stderr: {}",
                            String::from_utf8_lossy(&line).trim_end()
                        ),
                    }
                }
            });
        }

        let mut session = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            pending: Vec::new(),
            session_id: None,
            next_request_id: 1,
        };

        if options.use_default_args {
            let sdk_servers = if options.orchestrate {
                vec![crate::mcp::SERVER_NAME.to_string()]
            } else {
                Vec::new()
            };
            timeout(HANDSHAKE_TIMEOUT, session.handshake_initialize(sdk_servers))
                .await
                .map_err(|_| Error::Engine("timed out waiting for Claude to start".into()))??;
        }

        Ok(session)
    }

    async fn handshake_initialize(&mut self, sdk_servers: Vec<String>) -> Result<()> {
        let request_id = self.alloc_request_id("init");
        self.write_message(&initialize_request(&request_id, sdk_servers))
            .await?;
        loop {
            match self.next_event().await? {
                Some(Event::ControlResponse {
                    request_id: response_id,
                    ok,
                }) if response_id == request_id => {
                    if !ok {
                        return Err(Error::Engine("initialize handshake failed".into()));
                    }
                    return Ok(());
                }
                // The CLI connects its MCP servers before it answers the handshake.
                Some(Event::McpMessage {
                    request_id,
                    message,
                    ..
                }) => {
                    self.answer_mcp(&request_id, crate::mcp::handshake_reply(&message))
                        .await?;
                }
                Some(_) => continue,
                None => {
                    return Err(Error::Engine(
                        "claude exited during initialize handshake".into(),
                    ));
                }
            }
        }
    }

    #[cfg(test)]
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    #[cfg(test)]
    pub async fn send_user(&mut self, text: impl Into<String>) -> Result<()> {
        self.write_message(&user_message(text.into())).await
    }

    pub async fn send_prompt(&mut self, prompt: &Prompt) -> Result<()> {
        self.write_message(&user_prompt_message(prompt)).await
    }

    pub async fn answer_tool(
        &mut self,
        request_id: &str,
        decision: PermissionDecision,
    ) -> Result<()> {
        self.write_message(&permission_response(request_id, decision))
            .await
    }

    pub async fn answer_mcp(&mut self, request_id: &str, response: Value) -> Result<()> {
        self.write_message(&mcp_response(request_id, response))
            .await
    }

    pub async fn interrupt(&mut self) -> Result<String> {
        let request_id = self.alloc_request_id("int");
        self.write_message(&interrupt_request(&request_id)).await?;
        Ok(request_id)
    }

    pub async fn stop_task(&mut self, task_id: String) -> Result<()> {
        let request_id = self.alloc_request_id("stop");
        self.write_message(&stop_task_request(&request_id, task_id))
            .await
    }

    pub async fn set_model(&mut self, model: Option<String>) -> Result<String> {
        let request_id = self.alloc_request_id("model");
        self.write_message(&set_model_request(&request_id, model))
            .await?;
        Ok(request_id)
    }

    pub async fn set_effort(&mut self, effort: Option<String>) -> Result<String> {
        let request_id = self.alloc_request_id("effort");
        self.write_message(&set_effort_request(&request_id, effort))
            .await?;
        Ok(request_id)
    }

    /// Cancel safe: `read_until` keeps partial reads in `pending`, so the pump can race this
    /// against commands in `select!` without dropping half a line. `None` once stdout closes.
    pub async fn next_event(&mut self) -> Result<Option<Event>> {
        loop {
            let read = self
                .stdout
                .read_until(b'\n', &mut self.pending)
                .await
                .map_err(|error| Error::Engine(error.to_string()))?;
            if read == 0 && self.pending.is_empty() {
                return Ok(None);
            }
            let event = decode_line(&String::from_utf8_lossy(&self.pending));
            // Kept between lines, but one huge payload doesn't stay allocated.
            self.pending.clear();
            self.pending.shrink_to(LINE_BUFFER_KEEP);
            let Some(event) = event else {
                continue;
            };
            if let Event::Init { session_id, .. } = &event {
                self.session_id = Some(session_id.clone());
            }
            if let Event::Result { session_id, .. } = &event
                && let Some(id) = session_id
            {
                self.session_id = Some(id.clone());
            }
            return Ok(Some(event));
        }
    }

    #[cfg(test)]
    /// Drive until a turn ends (`result`) or a tool needs a decision.
    pub async fn wait_for_turn_or_tool(&mut self) -> Result<Event> {
        loop {
            match self.next_event().await? {
                Some(event @ (Event::CanUseTool(_) | Event::Result { .. })) => return Ok(event),
                Some(_) => continue,
                None => return Err(Error::Engine("claude exited before result".into())),
            }
        }
    }

    /// The CLI's pid, which is also its process group id (`process_group(0)`).
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Closes stdin, asks the CLI's whole process group to stop, and kills whatever is left
    /// after a few seconds. The group is the CLI's own (`process_group(0)`), so the shells and
    /// servers it started go with it rather than outliving the thread.
    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.stdin.shutdown().await;
        let pgid = self.child.id().map(|pid| pid as i32);
        if let Some(pgid) = pgid {
            let _ = unsafe { libc::killpg(pgid, libc::SIGTERM) };
        }
        let _ = timeout(Duration::from_secs(5), self.child.wait()).await;
        // The CLI exiting on SIGTERM doesn't mean its children did; `kill_on_drop` only
        // reaches the CLI itself.
        if let Some(pgid) = pgid {
            let _ = unsafe { libc::killpg(pgid, libc::SIGKILL) };
        }
        let _ = self.child.kill().await;
        Ok(())
    }

    async fn write_message(&mut self, message: &StdinMessage) -> Result<()> {
        let line = encode_line(message)?;
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| Error::Engine(error.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| Error::Engine(error.to_string()))?;
        Ok(())
    }

    fn alloc_request_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next_request_id);
        self.next_request_id += 1;
        id
    }
}

/// One line of the CLI's stdout as an event. Blank lines are skipped, and so is anything that
/// isn't an event (a stray print, a truncated payload): one bad line must not take the session
/// down with it, since the CLI keeps running and the next line is usually fine.
fn decode_line(line: &str) -> Option<Event> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    match decode_event(trimmed) {
        Ok(event) => Some(event),
        Err(error) => {
            let shown: String = trimmed.chars().take(200).collect();
            log::warn!("claude: skipping a line that isn't an event ({error}): {shown}");
            None
        }
    }
}

#[cfg(test)]
/// In-process NDJSON peer used by the spike tests (no real CLI).
pub struct NdjsonPeer<R, W> {
    writer: W,
    stdout: BufReader<R>,
    session_id: Option<String>,
    next_request_id: u64,
}

#[cfg(test)]
impl<R, W> NdjsonPeer<R, W>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            writer,
            stdout: BufReader::new(reader),
            session_id: None,
            next_request_id: 1,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub async fn send_user(&mut self, text: impl Into<String>) -> Result<()> {
        self.write_message(&user_message(text.into())).await
    }

    pub async fn answer_tool(
        &mut self,
        request_id: &str,
        decision: PermissionDecision,
    ) -> Result<()> {
        self.write_message(&permission_response(request_id, decision))
            .await
    }

    pub async fn interrupt(&mut self) -> Result<String> {
        let request_id = format!("int-{}", self.next_request_id);
        self.next_request_id += 1;
        self.write_message(&interrupt_request(&request_id)).await?;
        Ok(request_id)
    }

    pub async fn next_event(&mut self) -> Result<Option<Event>> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .stdout
                .read_line(&mut line)
                .await
                .map_err(|error| Error::Engine(error.to_string()))?;
            if read == 0 {
                return Ok(None);
            }
            let Some(event) = decode_line(&line) else {
                continue;
            };
            if let Event::Init { session_id, .. } = &event {
                self.session_id = Some(session_id.clone());
            }
            return Ok(Some(event));
        }
    }

    async fn write_message(&mut self, message: &StdinMessage) -> Result<()> {
        let line = encode_line(message)?;
        self.writer
            .write_all(line.as_bytes())
            .await
            .map_err(|error| Error::Engine(error.to_string()))?;
        self.writer
            .flush()
            .await
            .map_err(|error| Error::Engine(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
pub fn allow_input(input: Value) -> PermissionDecision {
    PermissionDecision::Allow {
        updated_input: input,
        updated_permissions: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};
    use tokio::time::timeout;

    async fn read_json_line(reader: &mut BufReader<impl tokio::io::AsyncRead + Unpin>) -> Value {
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        serde_json::from_str(line.trim_end()).unwrap()
    }

    #[test]
    fn a_line_that_is_not_an_event_is_skipped_not_fatal() {
        assert!(decode_line("").is_none());
        assert!(decode_line("   \n").is_none());
        assert!(decode_line("Warning: something the CLI printed").is_none());
        assert!(decode_line(r#"{"type":"system","subtype":"init"}"#).is_none());
        assert!(matches!(
            decode_line(r#"{"type":"system","subtype":"init","session_id":"s"}"#),
            Some(Event::Init { .. })
        ));
    }

    #[tokio::test]
    async fn a_bad_line_mid_stream_does_not_end_the_session() {
        let (_host_out, _mock_in) = duplex(4096);
        let (mut mock_out, host_in) = duplex(4096);
        let mut host = NdjsonPeer::new(host_in, _host_out);
        mock_out
            .write_all(b"not json at all\n{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false}\n")
            .await
            .unwrap();
        mock_out.flush().await.unwrap();
        assert!(matches!(
            host.next_event().await.unwrap(),
            Some(Event::Result {
                is_error: false,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn spike_two_messages_can_use_tool_interrupt_resume() {
        // Host writes to mock stdin; mock writes to host stdout.
        let (host_out, mock_in) = duplex(64 * 1024);
        let (mock_out, host_in) = duplex(64 * 1024);
        let mut host = NdjsonPeer::new(host_in, host_out);
        let mut mock_reader = BufReader::new(mock_in);
        let mut mock_writer = mock_out;

        mock_writer
            .write_all(br#"{"type":"system","subtype":"init","session_id":"sess-1"}"#)
            .await
            .unwrap();
        mock_writer.write_all(b"\n").await.unwrap();
        mock_writer.flush().await.unwrap();

        let init = host.next_event().await.unwrap().unwrap();
        assert_eq!(
            init,
            Event::Init {
                session_id: "sess-1".into(),
                permission_mode: None,
            }
        );
        assert_eq!(host.session_id(), Some("sess-1"));

        host.send_user("first: run echo spike-ok").await.unwrap();
        let first = read_json_line(&mut mock_reader).await;
        assert_eq!(first["type"], "user");
        assert_eq!(
            first["message"]["content"][0]["text"],
            "first: run echo spike-ok"
        );

        mock_writer
            .write_all(br#"{"type":"control_request","request_id":"req_bash","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"echo spike-ok"},"tool_use_id":"toolu_1"}}"#)
            .await
            .unwrap();
        mock_writer.write_all(b"\n").await.unwrap();
        mock_writer.flush().await.unwrap();

        let Event::CanUseTool(req) = host.next_event().await.unwrap().unwrap() else {
            panic!("expected can_use_tool");
        };
        assert_eq!(req.tool_name, "Bash");
        host.answer_tool(&req.request_id, allow_input(req.input.clone()))
            .await
            .unwrap();

        let allow = read_json_line(&mut mock_reader).await;
        assert_eq!(allow["type"], "control_response");
        assert_eq!(allow["response"]["request_id"], "req_bash");
        assert_eq!(allow["response"]["response"]["behavior"], "allow");
        assert_eq!(
            allow["response"]["response"]["updatedInput"]["command"],
            "echo spike-ok"
        );

        mock_writer
            .write_all(br#"{"type":"result","subtype":"success","session_id":"sess-1","is_error":false,"result":"ok"}"#)
            .await
            .unwrap();
        mock_writer.write_all(b"\n").await.unwrap();
        mock_writer.flush().await.unwrap();
        assert!(matches!(
            host.next_event().await.unwrap(),
            Some(Event::Result {
                is_error: false,
                ..
            })
        ));

        host.send_user("second: count to one thousand slowly")
            .await
            .unwrap();
        let second = read_json_line(&mut mock_reader).await;
        assert_eq!(
            second["message"]["content"][0]["text"],
            "second: count to one thousand slowly"
        );

        mock_writer
            .write_all(
                br#"{"type":"assistant","message":{"content":[{"type":"text","text":"1"}]}}"#,
            )
            .await
            .unwrap();
        mock_writer.write_all(b"\n").await.unwrap();
        mock_writer.flush().await.unwrap();
        assert!(matches!(
            host.next_event().await.unwrap(),
            Some(Event::Assistant { .. })
        ));

        let interrupt_id = host.interrupt().await.unwrap();
        let interrupt = read_json_line(&mut mock_reader).await;
        assert_eq!(interrupt["type"], "control_request");
        assert_eq!(interrupt["request"]["subtype"], "interrupt");
        assert_eq!(interrupt["request_id"], interrupt_id);

        mock_writer
            .write_all(
                format!(
                    r#"{{"type":"control_response","response":{{"subtype":"success","request_id":"{interrupt_id}"}}}}"#
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        mock_writer.write_all(b"\n").await.unwrap();
        mock_writer
            .write_all(
                br#"{"type":"result","subtype":"error","is_error":true,"session_id":"sess-1"}"#,
            )
            .await
            .unwrap();
        mock_writer.write_all(b"\n").await.unwrap();
        mock_writer.flush().await.unwrap();

        let mut saw_interrupt_ack = false;
        let mut saw_error_result = false;
        while let Some(event) = host.next_event().await.unwrap() {
            match event {
                Event::ControlResponse { request_id, ok } => {
                    assert_eq!(request_id, interrupt_id);
                    assert!(ok);
                    saw_interrupt_ack = true;
                }
                Event::Result { is_error, .. } => {
                    assert!(is_error);
                    saw_error_result = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_interrupt_ack);
        assert!(saw_error_result);

        // Resume is a new process with --resume <id>; prove the argv and a second init.
        let resume_args = crate::engines::claude_argv(host.session_id(), None, None, false, false);
        assert!(resume_args.windows(2).any(|w| w == ["--resume", "sess-1"]));

        let (host2_out, _mock2_in) = duplex(4096);
        let (mut mock2_out, host2_in) = duplex(4096);
        let mut host2 = NdjsonPeer::new(host2_in, host2_out);
        mock2_out
            .write_all(br#"{"type":"system","subtype":"init","session_id":"sess-1"}"#)
            .await
            .unwrap();
        mock2_out.write_all(b"\n").await.unwrap();
        mock2_out.flush().await.unwrap();
        let resumed = host2.next_event().await.unwrap().unwrap();
        assert_eq!(
            resumed,
            Event::Init {
                session_id: "sess-1".into(),
                permission_mode: None,
            }
        );
    }

    fn mock_python() -> SpawnOptions {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/claude/mock_claude.py");
        SpawnOptions {
            binary: PathBuf::from("python3"),
            extra_args: vec![script.to_string_lossy().into_owned()],
            use_default_args: false,
            ..SpawnOptions::default()
        }
    }

    #[tokio::test]
    async fn spawn_mock_cli_covers_control_protocol() {
        let mut session = ClaudeSession::spawn(mock_python()).await.unwrap();

        assert_eq!(
            session.next_event().await.unwrap(),
            Some(Event::Init {
                session_id: "sess-1".into(),
                permission_mode: None,
            })
        );

        session.send_user("first: run echo").await.unwrap();
        let Event::CanUseTool(req) = session.wait_for_turn_or_tool().await.unwrap() else {
            panic!("expected can_use_tool");
        };
        session
            .answer_tool(&req.request_id, allow_input(req.input.clone()))
            .await
            .unwrap();
        assert!(matches!(
            session.wait_for_turn_or_tool().await.unwrap(),
            Event::Result {
                is_error: false,
                ..
            }
        ));

        session.send_user("second: count slowly").await.unwrap();
        assert!(matches!(
            session.next_event().await.unwrap(),
            Some(Event::Assistant { .. })
        ));
        let interrupt_id = session.interrupt().await.unwrap();
        let mut saw_ack = false;
        loop {
            match session.next_event().await.unwrap() {
                Some(Event::ControlResponse { request_id, ok }) => {
                    assert_eq!(request_id, interrupt_id);
                    assert!(ok);
                    saw_ack = true;
                }
                Some(Event::Result { is_error, .. }) => {
                    assert!(is_error);
                    break;
                }
                Some(_) => {}
                None => panic!("mock exited early"),
            }
        }
        assert!(saw_ack);
        session.shutdown().await.unwrap();

        let mut resumed = ClaudeSession::spawn(SpawnOptions {
            resume: Some("sess-1".into()),
            extra_args: {
                let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("src/engines/claude/mock_claude.py");
                vec![script.to_string_lossy().into_owned()]
            },
            binary: PathBuf::from("python3"),
            use_default_args: false,
            ..SpawnOptions::default()
        })
        .await
        .unwrap();
        assert_eq!(
            resumed.next_event().await.unwrap(),
            Some(Event::Init {
                session_id: "sess-1".into(),
                permission_mode: None,
            })
        );
        resumed.send_user("continue").await.unwrap();
        assert!(matches!(
            resumed.wait_for_turn_or_tool().await.unwrap(),
            Event::Result {
                result: Some(text),
                ..
            } if text == "resumed"
        ));
        resumed.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "talks to a real `claude` binary; cargo test -- --ignored --nocapture"]
    async fn live_claude_control_protocol() {
        let cwd = std::env::temp_dir().join("cormux-cor-42-spike");
        std::fs::create_dir_all(&cwd).unwrap();

        let mut session = ClaudeSession::spawn(SpawnOptions {
            cwd: Some(cwd.clone()),
            extra_args: vec![
                "--max-turns".into(),
                "4".into(),
                "--permission-mode".into(),
                "plan".into(),
            ],
            ..SpawnOptions::default()
        })
        .await
        .expect("spawn claude");

        session
            .send_user(
                "Write a file named spike-ok.txt in the current directory containing the word ok.",
            )
            .await
            .unwrap();

        let first = timeout(Duration::from_secs(60), session.wait_for_turn_or_tool())
            .await
            .expect("timed out waiting for can_use_tool")
            .unwrap();
        let Event::CanUseTool(req) = first else {
            panic!("expected can_use_tool, got {first:?}");
        };
        eprintln!("can_use_tool {}", req.tool_name);
        session
            .answer_tool(&req.request_id, allow_input(req.input.clone()))
            .await
            .unwrap();

        session
            .send_user("Count slowly from 1 to 1000 in words, one number per sentence.")
            .await
            .unwrap();
        let _ = timeout(Duration::from_secs(20), session.next_event()).await;
        let interrupt_id = session.interrupt().await.unwrap();
        eprintln!("sent interrupt {interrupt_id}");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
        loop {
            if tokio::time::Instant::now() > deadline {
                break;
            }
            match timeout(Duration::from_secs(5), session.next_event()).await {
                Ok(Ok(Some(Event::Result { .. }) | None)) => break,
                Ok(Ok(Some(Event::CanUseTool(req)))) => {
                    let _ = session
                        .answer_tool(
                            &req.request_id,
                            PermissionDecision::Deny {
                                message: "interrupt spike".into(),
                            },
                        )
                        .await;
                }
                Ok(Ok(Some(Event::ControlResponse { request_id, ok }))) => {
                    eprintln!("interrupt ack {request_id} ok={ok}");
                }
                _ => continue,
            }
        }

        let session_id = session.session_id().unwrap_or("unknown").to_string();
        session.shutdown().await.unwrap();

        let mut resumed = ClaudeSession::spawn(SpawnOptions {
            cwd: Some(cwd),
            resume: Some(session_id.clone()),
            extra_args: vec!["--max-turns".into(), "1".into()],
            ..SpawnOptions::default()
        })
        .await
        .unwrap();
        eprintln!(
            "resumed original={session_id} new={:?}",
            resumed.session_id()
        );
        resumed
            .send_user("Reply with exactly the word resumed. Do not use tools.")
            .await
            .unwrap();
        let resumed_turn = timeout(Duration::from_secs(45), async {
            loop {
                match resumed.wait_for_turn_or_tool().await.unwrap() {
                    Event::CanUseTool(req) => {
                        resumed
                            .answer_tool(
                                &req.request_id,
                                PermissionDecision::Deny {
                                    message: "not needed".into(),
                                },
                            )
                            .await
                            .unwrap();
                    }
                    Event::Result { .. } => break,
                    other => panic!("unexpected {other:?}"),
                }
            }
        })
        .await;
        assert!(resumed_turn.is_ok(), "resume turn timed out");
        resumed.shutdown().await.unwrap();
    }
}
