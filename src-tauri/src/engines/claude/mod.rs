//! The Claude Code adapter: one `claude -p` per thread, spoken to over its stream-JSON control protocol.
//!
//! Findings against Claude Code 2.1.x:
//! - `-p` is required; `--input-format stream-json` is print-mode only.
//! - `--permission-prompt-tool stdio` is still what the official SDK passes
//!   (it is not listed in `--help`). Pair it with `--permission-prompts host`.
//! - `--await-initialize` plus a host `control_request`/`initialize` is required
//!   before the CLI treats us as a permission surface. Without that handshake,
//!   asks become `system`/`permission_denied` instead of `can_use_tool`.
//! - With stdin held open, `system`/`init` often arrives only after the first
//!   user message, not during the initialize handshake.
//! - Interrupt is a host `control_request` with `subtype: interrupt`.
//! - Resume is a new process with `--resume <session_id>` from `system`/`init`.
//! - An `sdk` server in `--mcp-config`, also named as `sdkMcpServers` in `initialize`, is
//!   answered by the host: each JSON-RPC message arrives as a `control_request` with
//!   `subtype: mcp_message`, and every one needs a `control_response`.
//! - A turn's `result` lands while background subagents run. `system`/`background_tasks_changed`
//!   lists what is live, and each agent that finishes starts a turn of its own with a new
//!   `system`/`init`. One stopped with `stop_task` starts none. Interrupt doesn't reach them.
//! - A prompt sent mid-turn is taken into that turn at its next tool call (one `result` answers
//!   both), or, if none comes, starts a turn of its own after the `result`, with a new
//!   `system`/`init`. `--replay-user-messages` echoes each prompt as a `user` message with
//!   `isReplay` when the CLI takes it, the only way to tell the two apart when the `result` lands.

mod map;
mod protocol;
pub mod saved;
mod session;
mod tools;
mod turn_gate;

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::{RwLock, mpsc, oneshot};

use self::protocol::{CanUseTool, Event, PermissionDecision};
use self::session::{ClaudeSession, SpawnOptions, ask_host};
use self::turn_gate::TurnGate;
use super::detect::{CLAUDE_BINARY, resolve_binary};
use super::{
    EngineCommand, EngineHandle, EngineOutput, Engines, LaunchConfig, Launching, PermissionAnswer,
    PermissionRequest,
};
use crate::model::StopReason;
use crate::shell_env::ShellEnv;

/// Launches `claude` from the login shell's PATH, in its environment.
pub struct ClaudeEngines {
    env: Arc<RwLock<ShellEnv>>,
}

impl ClaudeEngines {
    pub fn new(env: Arc<RwLock<ShellEnv>>) -> Self {
        Self { env }
    }
}

impl Engines for ClaudeEngines {
    fn launch(
        &self,
        config: LaunchConfig,
        output: mpsc::UnboundedSender<EngineOutput>,
    ) -> Launching<'_> {
        Box::pin(async move {
            let env = self.env.read().await.clone();
            let binary =
                resolve_binary(&env, CLAUDE_BINARY).unwrap_or_else(|| PathBuf::from(CLAUDE_BINARY));
            let options = SpawnOptions {
                label: config.thread_id.clone(),
                binary,
                args: argv(&config),
                cwd: Some(config.cwd.clone()),
                env: (!env.vars().is_empty()).then(|| env.vars().clone()),
                sdk_servers: config
                    .host_tools
                    .iter()
                    .map(|tools| tools.server_name.clone())
                    .collect(),
            };
            let session = ClaudeSession::spawn(options, &output).await?;
            let pgid = session.pid();
            let (commands, received) = mpsc::unbounded_channel();
            tokio::spawn(run(session, received, output));
            Ok(EngineHandle {
                commands,
                pgid,
                stops_within: ClaudeSession::STOPS_WITHIN,
            })
        })
    }
}

fn argv(config: &LaunchConfig) -> Vec<String> {
    let mut brief = config.instructions.clone();
    let mut disallowed = vec![tools::ENTER_PLAN_TOOL.to_string()];
    let mut host_args = Vec::new();
    if let Some(host) = &config.host_tools {
        let named = |tool: &String| tools::mcp_tool_name(&host.server_name, tool);
        if !host.instructions.is_empty() {
            brief.push_str("\n\n");
            brief.push_str(&host.instructions);
        }
        disallowed.extend(host.disallowed.iter().map(named));
        host_args = vec![
            "--mcp-config".into(),
            json!({ "mcpServers": { &host.server_name: { "type": "sdk", "name": host.server_name } } })
                .to_string(),
            "--allowedTools".into(),
            host.allowed.iter().map(named).collect::<Vec<_>>().join(","),
        ];
    }
    let mut args: Vec<String> = vec![
        "-p".into(),
        "--verbose".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--replay-user-messages".into(),
        "--permission-prompt-tool".into(),
        "stdio".into(),
        "--permission-prompts".into(),
        "host".into(),
        "--await-initialize".into(),
        "--append-system-prompt".into(),
        brief,
    ];
    args.extend(host_args);
    args.push("--disallowedTools".into());
    args.push(disallowed.join(","));
    for (flag, value) in [
        ("--resume", &config.resume),
        ("--model", &config.model),
        ("--effort", &config.effort),
    ] {
        if let Some(value) = value {
            args.push(flag.into());
            args.push(value.clone());
        }
    }
    args
}

/// One session's loop: commands in, the CLI's events out as engine-neutral output, in order.
async fn run(
    mut session: ClaudeSession,
    mut commands: mpsc::UnboundedReceiver<EngineCommand>,
    output: mpsc::UnboundedSender<EngineOutput>,
) {
    // Answers come back off the loop, which has to keep reading while the host decides.
    let (answered, mut answers) = mpsc::unbounded_channel::<(String, PermissionDecision)>();
    let (replied, mut replies) = mpsc::unbounded_channel::<(String, Value)>();
    let mut process_cost = 0.0_f64;
    let mut gate = TurnGate::default();
    loop {
        let release_at = gate.release_at();
        tokio::select! {
            command = commands.recv() => match command {
                Some(EngineCommand::Prompt(prompt)) => match session.send_prompt(&prompt).await {
                    Ok(()) => gate.prompted(),
                    Err(error) => log::warn!("claude prompt: {error}"),
                },
                Some(EngineCommand::Cancel) => {
                    if let Err(error) = session.interrupt().await {
                        log::warn!("claude interrupt: {error}");
                    }
                    gate.interrupted();
                    for task_id in gate.running_agents() {
                        if let Err(error) = session.stop_task(task_id).await {
                            log::warn!("claude stop task: {error}");
                        }
                    }
                    if let Some(held) = gate.release() {
                        end_turn(&output, held, gate.queued());
                    }
                }
                Some(EngineCommand::SetModel(model)) => {
                    if let Err(error) = session.set_model(model).await {
                        log::warn!("claude set model: {error}");
                    }
                }
                Some(EngineCommand::SetEffort(effort)) => {
                    if let Err(error) = session.set_effort(effort).await {
                        log::warn!("claude set effort: {error}");
                    }
                }
                Some(EngineCommand::Shutdown) | None => {
                    session.shutdown().await;
                    break;
                }
            },
            event = session.next_event() => match event {
                Ok(Some(raw)) => {
                    if let Some((used_tokens, context_size, process_total)) =
                        map::usage_from_result(&raw)
                    {
                        let turn_cost_usd = process_total.map(|total| {
                            let (cost, seen) = map::turn_cost(process_cost, total);
                            process_cost = seen;
                            cost
                        });
                        let _ = output.send(EngineOutput::Usage {
                            used_tokens,
                            context_size,
                            turn_cost_usd,
                        });
                    }
                    match raw {
                        Event::McpMessage { request_id, message, server_name } => {
                            let (host, replied) = (output.clone(), replied.clone());
                            tokio::spawn(async move {
                                let reply = ask_host(&host, server_name, message).await;
                                let _ = replied.send((request_id, reply));
                            });
                        }
                        Event::CanUseTool(request) => {
                            ask_permission(&output, request, answered.clone());
                        }
                        other => {
                            gate.observe(&other);
                            for mapped in map::map_claude_event(&other) {
                                let _ = output.send(EngineOutput::Event(mapped));
                            }
                            if let Some(end) = map::turn_end(&other).and_then(|end| gate.pass(end)) {
                                end_turn(&output, end, gate.queued());
                            }
                        }
                    }
                }
                Ok(None) => {
                    let _ = output.send(EngineOutput::Exited);
                    // The CLI's gone, but the shells and servers it started are in its
                    // process group and outlive it unless the group is killed too.
                    session.shutdown().await;
                    break;
                }
                Err(error) => {
                    // Saved, so after a reload the thread still says why its turn ended.
                    end_turn(
                        &output,
                        map::TurnEnd {
                            stop_reason: StopReason::failed(None),
                            error: Some(error.to_string()),
                        },
                        false,
                    );
                    session.shutdown().await;
                    break;
                }
            },
            _ = tokio::time::sleep_until(release_at.unwrap_or_else(tokio::time::Instant::now)), if release_at.is_some() => {
                if let Some(held) = gate.release() {
                    end_turn(&output, held, gate.queued());
                }
            }
            Some((request_id, reply)) = replies.recv() => {
                if let Err(error) = session.answer_mcp(&request_id, reply).await {
                    log::warn!("claude mcp answer: {error}");
                }
            }
            Some((request_id, decision)) = answers.recv() => {
                // A failed write leaves the CLI waiting on this tool for good.
                if let Err(error) = session.answer_tool(&request_id, decision).await {
                    log::warn!("claude permission answer: {error}");
                }
            }
        }
    }
}

/// A turn's end carries whether the CLI already has the next turn's prompt.
fn end_turn(output: &mpsc::UnboundedSender<EngineOutput>, end: map::TurnEnd, queued: bool) {
    let _ = output.send(EngineOutput::TurnEnded {
        stop_reason: end.stop_reason,
        error: end.error,
        next_queued: queued,
    });
}

/// Puts a permission request to the host and sends its answer back to the CLI. An answer that
/// never comes means the turn ended, and the CLI stopped waiting for one.
fn ask_permission(
    output: &mpsc::UnboundedSender<EngineOutput>,
    request: CanUseTool,
    answered: mpsc::UnboundedSender<(String, PermissionDecision)>,
) {
    let tool = tools::tool_use(&request.tool_name, &request.input);
    let questions = if request.tool_name == tools::QUESTION_TOOL {
        request
            .input
            .get("questions")
            .cloned()
            .and_then(|questions| serde_json::from_value(questions).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let detail = tool.asks_about();
    let (respond, answer) = oneshot::channel();
    let asked = output.send(EngineOutput::Permission(PermissionRequest {
        id: request.request_id.clone(),
        tool_call_id: request.tool_use_id.clone(),
        tool: tools::display_name(&request.tool_name),
        role: tool.role,
        detail,
        questions,
        respond,
    }));
    if asked.is_err() {
        return;
    }
    tokio::spawn(async move {
        if let Ok(answer) = answer.await {
            let decision = permission_decision(answer, request.input);
            let _ = answered.send((request.request_id, decision));
        }
    });
}

/// The CLI wants the tool's input back with an approval, with the answers added for a question.
fn permission_decision(answer: PermissionAnswer, input: Value) -> PermissionDecision {
    match answer {
        PermissionAnswer::Allow => PermissionDecision::Allow {
            updated_input: input,
        },
        PermissionAnswer::Deny { message } => PermissionDecision::Deny { message },
        PermissionAnswer::Answer { answers } => {
            let mut updated_input = input;
            updated_input["answers"] = json!(answers);
            PermissionDecision::Allow { updated_input }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::HostToolsConfig;

    fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
        let at = args.iter().position(|arg| arg == name)?;
        args.get(at + 1).map(String::as_str)
    }

    #[test]
    fn argv_includes_print_and_stdio_permission_tool() {
        let args = argv(&LaunchConfig::default());
        assert!(args.contains(&"-p".into()));
        assert_eq!(flag(&args, "--permission-prompt-tool"), Some("stdio"));
        assert!(args.contains(&"--await-initialize".into()));
        assert!(args.contains(&"--replay-user-messages".into()));
        assert_eq!(flag(&args, "--disallowedTools"), Some("EnterPlanMode"));
        assert_eq!(flag(&args, "--mcp-config"), None);
    }

    #[test]
    fn host_tools_get_their_server_names_and_brief() {
        let args = argv(&LaunchConfig {
            instructions: "Product brief.".into(),
            host_tools: Some(HostToolsConfig {
                server_name: "cormux".into(),
                allowed: vec!["get_thread".into()],
                disallowed: vec!["attach_entity".into()],
                instructions: "Tools brief.".into(),
            }),
            ..LaunchConfig::default()
        });
        assert!(
            flag(&args, "--mcp-config")
                .unwrap()
                .contains(r#""type":"sdk""#)
        );
        assert_eq!(
            flag(&args, "--allowedTools"),
            Some("mcp__cormux__get_thread")
        );
        assert_eq!(
            flag(&args, "--disallowedTools"),
            Some("EnterPlanMode,mcp__cormux__attach_entity")
        );
        assert_eq!(
            flag(&args, "--append-system-prompt"),
            Some("Product brief.\n\nTools brief.")
        );
    }

    #[test]
    fn argv_carries_the_session_model_and_effort() {
        let args = argv(&LaunchConfig {
            resume: Some("sess-1".into()),
            model: Some("opus".into()),
            effort: Some("high".into()),
            ..LaunchConfig::default()
        });
        assert_eq!(flag(&args, "--resume"), Some("sess-1"));
        assert_eq!(flag(&args, "--model"), Some("opus"));
        assert_eq!(flag(&args, "--effort"), Some("high"));
    }

    #[test]
    fn an_answered_question_goes_back_with_the_original_input() {
        let input = json!({ "questions": [{ "question": "Which colour?", "header": "Colour" }] });
        let answers = [("Which colour?".to_string(), "Green".to_string())].into();
        let PermissionDecision::Allow { updated_input, .. } =
            permission_decision(PermissionAnswer::Answer { answers }, input)
        else {
            panic!("an answer allows the tool");
        };
        assert_eq!(updated_input["answers"]["Which colour?"], "Green");
        assert_eq!(updated_input["questions"][0]["header"], "Colour");
    }
}
