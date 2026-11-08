use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

#[cfg(test)]
use super::protocol::user_message;
use super::protocol::{
    Event, PermissionDecision, StdinMessage, decode_event, encode_line, initialize_request,
    interrupt_request, mcp_response, permission_response, set_effort_request, set_model_request,
    stop_task_request, user_prompt_message,
};
use crate::engines::prompt::Prompt;
use crate::engines::{EngineOutput, HostToolCall};
use crate::error::{Error, Result};

/// How long the CLI gets to answer the initialize handshake. Once a session is up there is
/// no read timeout: Claude is silent between turns and while a long tool runs.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct SpawnOptions {
    /// Names the session in its log lines.
    pub label: String,
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    /// Replaces the app's environment when given.
    pub env: Option<HashMap<String, String>>,
    /// MCP servers the host answers itself, named in the initialize handshake.
    pub sdk_servers: Vec<String>,
}

const LINE_BUFFER_KEEP: usize = 1024 * 1024;

/// One long-lived `claude -p` process speaking NDJSON on stdin/stdout.
pub struct ClaudeSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// The line being read. It lives here, not in `next_event`, so a read cancelled by
    /// `select!` keeps what it had and the next call finishes the same line.
    pending: Vec<u8>,
    next_request_id: u64,
}

impl ClaudeSession {
    /// Starts the CLI and waits for it to take the initialize handshake. MCP messages it sends
    /// meanwhile go to the host through `host`.
    pub async fn spawn(
        options: SpawnOptions,
        host: &mpsc::UnboundedSender<EngineOutput>,
    ) -> Result<Self> {
        let mut command = Command::new(&options.binary);
        command
            .args(&options.args)
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
        command.env("CLAUDE_CODE_ENTRYPOINT", "sdk-ts");
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
            let label = options.label.clone();
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
                            "claude stderr ({label}): {}",
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
            next_request_id: 1,
        };

        timeout(
            HANDSHAKE_TIMEOUT,
            session.handshake_initialize(options.sdk_servers, host),
        )
        .await
        .map_err(|_| Error::Engine("timed out waiting for Claude to start".into()))??;
        Ok(session)
    }

    async fn handshake_initialize(
        &mut self,
        sdk_servers: Vec<String>,
        host: &mpsc::UnboundedSender<EngineOutput>,
    ) -> Result<()> {
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
                    server_name,
                }) => {
                    let reply = ask_host(host, server_name, message).await;
                    self.answer_mcp(&request_id, reply).await?;
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

    pub async fn interrupt(&mut self) -> Result<()> {
        let request_id = self.alloc_request_id("int");
        self.write_message(&interrupt_request(&request_id)).await
    }

    pub async fn stop_task(&mut self, task_id: String) -> Result<()> {
        let request_id = self.alloc_request_id("stop");
        self.write_message(&stop_task_request(&request_id, task_id))
            .await
    }

    pub async fn set_model(&mut self, model: Option<String>) -> Result<()> {
        let request_id = self.alloc_request_id("model");
        self.write_message(&set_model_request(&request_id, model))
            .await
    }

    pub async fn set_effort(&mut self, effort: Option<String>) -> Result<()> {
        let request_id = self.alloc_request_id("effort");
        self.write_message(&set_effort_request(&request_id, effort))
            .await
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
    pub async fn shutdown(&mut self) {
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

/// Hands one MCP message to the host and waits for its reply. A host that has gone away gets the
/// CLI an error rather than a hang.
pub async fn ask_host(
    host: &mpsc::UnboundedSender<EngineOutput>,
    server: String,
    message: Value,
) -> Value {
    let id = message.get("id").cloned();
    let (respond, reply) = oneshot::channel();
    let asked = host.send(EngineOutput::HostTool(HostToolCall {
        server,
        message,
        respond,
    }));
    match (asked, reply.await) {
        (Ok(()), Ok(reply)) => reply,
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32603, "message": "Cormux is not answering" },
        }),
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
pub fn allow_input(input: Value) -> PermissionDecision {
    PermissionDecision::Allow {
        updated_input: input,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

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

    /// A CLI that takes the handshake, prints a line that isn't an event, then ends a turn.
    const NOISY_CLI: &str = r#"
import json, sys
init = json.loads(sys.stdin.readline())
print(json.dumps({"type": "control_response", "response": {"subtype": "success", "request_id": init["request_id"]}}), flush=True)
print("not json at all", flush=True)
print(json.dumps({"type": "result", "subtype": "success", "is_error": False}), flush=True)
sys.stdin.readline()
"#;

    #[tokio::test]
    async fn a_bad_line_mid_stream_does_not_end_the_session() {
        let options = SpawnOptions {
            label: "noisy".into(),
            binary: PathBuf::from("python3"),
            args: vec!["-c".into(), NOISY_CLI.into()],
            cwd: None,
            env: None,
            sdk_servers: Vec::new(),
        };
        let mut session = ClaudeSession::spawn(options, &host()).await.unwrap();

        assert!(matches!(
            session.next_event().await.unwrap(),
            Some(Event::Result {
                is_error: false,
                ..
            })
        ));
        session.shutdown().await;
    }

    fn mock_python() -> SpawnOptions {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engines/claude/mock_claude.py");
        SpawnOptions {
            label: "mock".into(),
            binary: PathBuf::from("python3"),
            args: vec![script.to_string_lossy().into_owned()],
            cwd: None,
            env: None,
            sdk_servers: Vec::new(),
        }
    }

    fn host() -> mpsc::UnboundedSender<EngineOutput> {
        mpsc::unbounded_channel().0
    }

    #[tokio::test]
    async fn spawn_mock_cli_covers_control_protocol() {
        let mut session = ClaudeSession::spawn(mock_python(), &host()).await.unwrap();

        assert_eq!(
            session.next_event().await.unwrap(),
            Some(Event::Init {
                session_id: "sess-1".into(),
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
        session.interrupt().await.unwrap();
        let mut saw_ack = false;
        loop {
            match session.next_event().await.unwrap() {
                Some(Event::ControlResponse { request_id, ok }) => {
                    assert!(request_id.starts_with("int-"), "{request_id}");
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
        session.shutdown().await;

        // The mock answers a resumed session the same way.
        let mut resumed = ClaudeSession::spawn(mock_python(), &host()).await.unwrap();
        assert_eq!(
            resumed.next_event().await.unwrap(),
            Some(Event::Init {
                session_id: "sess-1".into(),
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
        resumed.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "talks to a real `claude` binary; cargo test -- --ignored --nocapture"]
    async fn live_claude_control_protocol() {
        let cwd = std::env::temp_dir().join("cormux-live-claude");
        std::fs::create_dir_all(&cwd).unwrap();

        let live = |resume: Option<String>, extra: &[&str]| {
            let mut args = super::super::argv(&crate::engines::LaunchConfig {
                cwd: cwd.clone(),
                resume,
                ..Default::default()
            });
            args.extend(extra.iter().map(|arg| arg.to_string()));
            SpawnOptions {
                label: "live".into(),
                binary: PathBuf::from("claude"),
                args,
                cwd: Some(cwd.clone()),
                env: None,
                sdk_servers: Vec::new(),
            }
        };
        let mut session = ClaudeSession::spawn(
            live(None, &["--max-turns", "4", "--permission-mode", "plan"]),
            &host(),
        )
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
        session.interrupt().await.unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
        let mut session_id = None;
        loop {
            if tokio::time::Instant::now() > deadline {
                break;
            }
            match timeout(Duration::from_secs(5), session.next_event()).await {
                Ok(Ok(Some(Event::Result { session_id: id, .. }))) => {
                    session_id = id;
                    break;
                }
                Ok(Ok(None)) => break,
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

        let session_id = session_id.unwrap_or_else(|| "unknown".into());
        session.shutdown().await;

        let mut resumed = ClaudeSession::spawn(
            live(Some(session_id.clone()), &["--max-turns", "1"]),
            &host(),
        )
        .await
        .unwrap();
        eprintln!("resumed {session_id}");
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
        resumed.shutdown().await;
    }
}
