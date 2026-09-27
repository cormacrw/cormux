use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CancelNotification, ContentBlock, InitializeRequest, LoadSessionRequest, NewSessionRequest,
    PromptRequest, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionId, SessionNotification, TextContent,
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, ConnectionTo};
use tokio::sync::{broadcast, mpsc};

use super::map::{
    map_permission_request, map_session_update, map_stop_reason, pick_permission_option,
};
use crate::approvals::{ApprovalBroker, ApprovalDecision};
use crate::engines::command::EngineCommand;
use crate::engines::events::AgentEvent;
use crate::error::{Error, Result};
use crate::shell_env::ShellEnv;
use crate::store::Store;

pub struct AcpSpawn {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub resume: Option<String>,
    pub env: ShellEnv,
    pub events: broadcast::Sender<AgentEvent>,
    pub approvals: Arc<ApprovalBroker>,
    pub auto_approve_readonly: bool,
    pub session_id: Arc<std::sync::Mutex<Option<String>>>,
    pub store: Store,
    pub thread_id: String,
    pub pending_seed: Arc<std::sync::Mutex<Option<String>>>,
}

pub fn start(spawn: AcpSpawn) -> mpsc::UnboundedSender<EngineCommand> {
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        if let Err(error) = run(spawn, rx).await {
            log::warn!("acp session ended: {error}");
        }
    });
    tx
}

async fn run(spawn: AcpSpawn, mut commands: mpsc::UnboundedReceiver<EngineCommand>) -> Result<()> {
    let mut argv = spawn.argv.into_iter();
    let program = argv
        .next()
        .ok_or_else(|| Error::Engine("acp argv is empty".into()))?;
    let mut config = AcpAgentConfig::new(program).args(argv);
    if !spawn.env.vars().is_empty() {
        config = config.envs(spawn.env.vars().clone());
    }
    let agent = AcpAgent::new(config);
    let events = spawn.events.clone();
    let approvals = spawn.approvals.clone();
    let auto_ro = spawn.auto_approve_readonly;
    let stored_id = spawn.session_id.clone();
    let cwd = spawn.cwd.clone();
    let resume = spawn.resume.clone();
    let store = spawn.store.clone();
    let thread_id = spawn.thread_id.clone();
    let pending_seed = spawn.pending_seed.clone();

    agent_client_protocol::Client
        .builder()
        .name("cormux")
        .on_receive_notification(
            {
                let events = events.clone();
                let store_notify = store.clone();
                let thread_notify = thread_id.clone();
                async move |notification: SessionNotification, _cx| {
                    if let Some(event) = map_session_update(&notification.update) {
                        persist_event(&store_notify, &thread_notify, &event);
                        if let AgentEvent::Usage {
                            used_tokens,
                            context_size,
                            cost_usd,
                        } = &event
                        {
                            let _ = store_notify.set_thread_usage(
                                &thread_notify,
                                *used_tokens,
                                *context_size,
                                *cost_usd,
                            );
                        }
                        let _ = events.send(event);
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            {
                let events = events.clone();
                let approvals = approvals.clone();
                async move |request: RequestPermissionRequest, responder, _cx| {
                    let mut event = map_permission_request(&request);
                    let readonly = matches!(
                        &event,
                        AgentEvent::Permission { kind, .. } if kind.is_readonly()
                    );
                    if auto_ro && readonly {
                        if let AgentEvent::Permission { auto_approved, .. } = &mut event {
                            *auto_approved = true;
                        }
                        let _ = events.send(event);
                        respond_permission(&request, responder, true);
                        return Ok(());
                    }
                    let approval_id = match &event {
                        AgentEvent::Permission { id, .. } => id.clone(),
                        _ => request.tool_call.tool_call_id.to_string(),
                    };
                    let _ = events.send(event);
                    let decision =
                        approvals
                            .wait(approval_id)
                            .await
                            .unwrap_or(ApprovalDecision::Denied {
                                message: "approval dropped".into(),
                            });
                    respond_permission(
                        &request,
                        responder,
                        matches!(decision, ApprovalDecision::Approved),
                    );
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, {
            let events = events.clone();
            move |connection: ConnectionTo<Agent>| async move {
                connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await?;
                let session_id = if let Some(existing) = resume {
                    let id = SessionId::new(existing.clone());
                    match connection
                        .send_request(LoadSessionRequest::new(id.clone(), cwd.as_path()))
                        .block_task()
                        .await
                    {
                        Ok(_) => id,
                        Err(error) => {
                            log::warn!("session/load failed, starting a new session: {error}");
                            let summary =
                                store.transcript_summary(&thread_id, 40).unwrap_or_default();
                            if !summary.is_empty() {
                                *pending_seed.lock().unwrap() = Some(summary);
                            }
                            let _ = store.set_thread_readonly(&thread_id, true);
                            connection
                                .send_request(NewSessionRequest::new(cwd.clone()))
                                .block_task()
                                .await?
                                .session_id
                        }
                    }
                } else {
                    let created = connection
                        .send_request(NewSessionRequest::new(cwd.clone()))
                        .block_task()
                        .await?;
                    created.session_id
                };
                {
                    let id_str = session_id.to_string();
                    *stored_id.lock().unwrap() = Some(id_str.clone());
                    let _ = store.set_thread_session(&thread_id, &id_str);
                    let _ = events.send(AgentEvent::SessionStarted { session_id: id_str });
                }

                while let Some(command) = commands.recv().await {
                    match command {
                        EngineCommand::Prompt(text) => {
                            let wait = connection
                                .send_request(PromptRequest::new(
                                    session_id.clone(),
                                    vec![ContentBlock::Text(TextContent::new(text))],
                                ))
                                .block_task();
                            tokio::pin!(wait);
                            let result = loop {
                                tokio::select! {
                                    result = &mut wait => break result,
                                    command = commands.recv() => {
                                        match command {
                                            Some(EngineCommand::Cancel) => {
                                                let _ = connection.send_notification(
                                                    CancelNotification::new(session_id.clone()),
                                                );
                                            }
                                            Some(EngineCommand::Shutdown) | None => {
                                                let _ = connection.send_notification(
                                                    CancelNotification::new(session_id.clone()),
                                                );
                                                return Ok(());
                                            }
                                            Some(EngineCommand::Prompt(_)) => {}
                                        }
                                    }
                                }
                            };
                            match result {
                                Ok(response) => {
                                    let _ = events.send(map_stop_reason(&response.stop_reason));
                                }
                                Err(error) => {
                                    let _ = events.send(AgentEvent::TurnEnd {
                                        stop_reason: "error".into(),
                                        error: Some(error.to_string()),
                                    });
                                }
                            }
                        }
                        EngineCommand::Cancel => {
                            let _ = connection
                                .send_notification(CancelNotification::new(session_id.clone()));
                        }
                        EngineCommand::Shutdown => break,
                    }
                }
                Ok(())
            }
        })
        .await
        .map_err(|error| Error::Engine(error.to_string()))?;

    let _ = events.send(AgentEvent::EngineExited { code: None });
    Ok(())
}

fn persist_event(store: &Store, thread_id: &str, event: &AgentEvent) {
    let kind = match event {
        AgentEvent::SessionStarted { .. } => "session",
        AgentEvent::MessageChunk { .. } => "message",
        AgentEvent::ToolCall { .. } => "tool",
        AgentEvent::Plan { .. } => "plan",
        AgentEvent::Permission { .. } => "permission",
        AgentEvent::CurrentTool { .. } => "current_tool",
        AgentEvent::Usage { .. } => "usage",
        AgentEvent::TurnEnd { .. } => "turn_end",
        AgentEvent::EngineExited { .. } => "exit",
    };
    if let Ok(payload) = serde_json::to_string(event) {
        let _ = store.append_event(thread_id, kind, &payload);
    }
}

fn respond_permission(
    request: &RequestPermissionRequest,
    responder: agent_client_protocol::Responder<RequestPermissionResponse>,
    approved: bool,
) {
    if let Some(option_id) = pick_permission_option(request, approved) {
        let _ = responder.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id)),
        ));
    } else {
        let _ = responder.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Cancelled,
        ));
    }
}
