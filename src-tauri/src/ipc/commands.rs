use tauri::{State, ipc::Channel};

use crate::{error::Result, state::AppState};

use super::types::{AgentChunk, DiffUpdate, PtyChunk, Snapshot};

#[tauri::command]
#[specta::specta]
pub fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    Ok(Snapshot {
        version: state.snapshot_version(),
        view: super::types::AppView::Homebase,
    })
}

/// High-volume ordered stream of agent message chunks for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_chunks(thread_id: String, channel: Channel<AgentChunk>) -> Result<()> {
    let _ = (thread_id, channel);
    Ok(())
}

/// High-volume ordered stream of PTY output for a workspace's run/setup log.
#[tauri::command]
#[specta::specta]
pub fn subscribe_pty(workspace_id: String, channel: Channel<PtyChunk>) -> Result<()> {
    let _ = (workspace_id, channel);
    Ok(())
}

/// High-volume ordered stream of live diff updates for a worktree.
#[tauri::command]
#[specta::specta]
pub fn subscribe_diffs(workspace_id: String, channel: Channel<DiffUpdate>) -> Result<()> {
    let _ = (workspace_id, channel);
    Ok(())
}

/// Spike 5: stream agent chunks (~60hz) and PTY lines (100/s) for a few seconds.
#[tauri::command]
#[specta::specta]
pub fn start_streaming_spike(agent: Channel<AgentChunk>, pty: Channel<PtyChunk>) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        let started = std::time::Instant::now();
        let mut n = 0u32;
        while started.elapsed() < std::time::Duration::from_secs(6) {
            n += 1;
            let _ = agent.send(AgentChunk {
                thread_id: "spike".into(),
                text: format!("token-{n} "),
            });
            let _ = pty.send(PtyChunk {
                workspace_id: "spike".into(),
                line: format!("[{n}] app log line at 100hz"),
            });
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });
    Ok(())
}
