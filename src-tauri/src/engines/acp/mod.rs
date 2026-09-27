//! Spike 2 (COR-43) plus the long-lived Cursor/Gemini/Codex ACP session (COR-59).

mod map;
mod session;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    ContentBlock, InitializeRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, TextContent,
};
use agent_client_protocol::{AcpAgent, Agent, ConnectionTo};

pub use session::{AcpSpawn, start};

use crate::error::{Error, Result};

pub async fn permission_and_load<I, S>(args: I) -> Result<bool>
where
    I: IntoIterator<Item = S>,
    S: ToString,
{
    let agent = AcpAgent::from_args(args).map_err(|error| Error::Engine(error.to_string()))?;
    let approved = Arc::new(AtomicBool::new(false));
    let approved_cb = approved.clone();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    agent_client_protocol::Client
        .builder()
        .name("cormux-spike")
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _cx| {
                approved_cb.store(true, Ordering::SeqCst);
                let option_id = request.options.first().map(|opt| opt.option_id.clone());
                if let Some(id) = option_id {
                    let _ = responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)),
                    ));
                } else {
                    let _ = responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ));
                }
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, {
            let approved = approved.clone();
            move |connection: ConnectionTo<Agent>| async move {
                connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await?;
                let created = connection
                    .send_request(NewSessionRequest::new(cwd.clone()))
                    .block_task()
                    .await?;
                connection
                    .send_request(PromptRequest::new(
                        created.session_id.clone(),
                        vec![ContentBlock::Text(TextContent::new(
                            "touch spike-ok.txt".to_string(),
                        ))],
                    ))
                    .block_task()
                    .await?;
                connection
                    .send_request(LoadSessionRequest::new(
                        created.session_id.clone(),
                        cwd.as_path(),
                    ))
                    .block_task()
                    .await?;
                if !approved.load(Ordering::SeqCst) {
                    return Err(agent_client_protocol::Error::internal_error());
                }
                Ok(())
            }
        })
        .await
        .map_err(|error| Error::Engine(error.to_string()))?;

    Ok(approved.load(Ordering::SeqCst))
}

pub fn mock_args() -> Vec<String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/engines/acp/mock_acp.py");
    vec!["python3".into(), script.display().to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_agent_permission_round_trip_and_session_load() {
        assert!(permission_and_load(mock_args()).await.unwrap());
    }

    #[tokio::test]
    #[ignore = "talks to a real `agent acp` binary"]
    async fn live_cursor_agent_acp() {
        permission_and_load(["agent", "acp"]).await.unwrap();
    }
}
