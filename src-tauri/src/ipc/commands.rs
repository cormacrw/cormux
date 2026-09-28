use std::time::Duration;

use tauri::{AppHandle, State, ipc::Channel};
use tauri_specta::Event;

use crate::error::Result;
use crate::feedback::{emit_approval_counts, emit_toast, toast_for_approval};
use crate::ipc::events::{StateChanged, WorkspaceStatusChanged};
use crate::ipc::types::StateChangeKind;
use crate::state::AppState;

use super::types::{AgentChunk, DiffUpdate, PtyChunk, Snapshot, WorkspaceSummaryResult};

#[tauri::command]
#[specta::specta]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let trees = process_trees(&state);
    let memory = state.metrics.sample(&trees).ok();
    Ok(Snapshot {
        version: state.snapshot_version(),
        view: super::types::AppView::Homebase,
        persisted: state.store.snapshot()?,
        workspaces: state.workspace.list().await,
        memory,
        pending_live_approvals: state.approvals.pending_count(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn reload_environment(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.shell_env.write().await.reload().await?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::Environment,
    }
    .emit(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn fetch_on_focus(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let updates = state.fetch.tick().await?;
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::BehindCounts,
    }
    .emit(&app);
    let _ = updates;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_metrics(state: State<'_, AppState>) -> Result<crate::metrics::MemorySample> {
    state.metrics.sample(&process_trees(&state))
}

/// High-volume ordered stream of agent message chunks for one thread.
#[tauri::command]
#[specta::specta]
pub fn subscribe_agent_chunks(
    thread_id: String,
    channel: Channel<AgentChunk>,
    state: State<'_, AppState>,
) -> Result<()> {
    let engines = state.engines.clone();
    tauri::async_runtime::spawn(async move {
        let rx = loop {
            match engines.subscribe(&thread_id) {
                Ok(rx) => break rx,
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        };
        let mut rx = rx;
        while let Ok(event) = rx.recv().await {
            if let crate::engines::AgentEvent::MessageChunk { text, .. } = event {
                let _ = channel.send(AgentChunk {
                    thread_id: thread_id.clone(),
                    text,
                });
            }
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn detect_engines(
    state: State<'_, AppState>,
) -> Result<Vec<crate::engines::EngineStatus>> {
    state.engines.detect().await
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_approval(
    app: AppHandle,
    id: String,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    let tool = state
        .store
        .snapshot()?
        .approvals
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.tool.clone())
        .unwrap_or_default();
    state.approvals.resolve(&id, approved).await?;
    emit_approval_counts(&app, &state);
    if let Some(payload) = toast_for_approval(&tool, approved) {
        emit_toast(&app, payload);
    }
    Ok(())
}

/// High-volume ordered stream of PTY output for a workspace's run/setup log.
#[tauri::command]
#[specta::specta]
pub fn subscribe_pty(
    workspace_id: String,
    channel: Channel<PtyChunk>,
    state: State<'_, AppState>,
) -> Result<()> {
    let process = state.process.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let lines = process.drain_pending(&workspace_id);
            for line in lines {
                let _ = channel.send(PtyChunk {
                    workspace_id: workspace_id.clone(),
                    line,
                });
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });
    Ok(())
}

/// High-volume ordered stream of live diff updates for a worktree.
#[tauri::command]
#[specta::specta]
pub fn subscribe_diffs(
    workspace_id: String,
    channel: Channel<DiffUpdate>,
    state: State<'_, AppState>,
) -> Result<()> {
    let diffs = state.diffs.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        loop {
            if let Some(diff) = diffs.latest(&workspace_id) {
                if last.as_ref() != Some(&diff) {
                    let _ = channel.send(DiffUpdate {
                        workspace_id: workspace_id.clone(),
                        path: String::new(),
                        diff: Some(diff.clone()),
                    });
                    last = Some(diff);
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    Ok(())
}

/// Generate or return a cached workspace card summary (debounced LLM + local fallback).
#[tauri::command]
#[specta::specta]
pub async fn summarise_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceSummaryResult> {
    use crate::error::Error;
    use crate::summaries::{self, SUMMARY_MODEL_LABEL};

    let snapshot = state.store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == workspace_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    if let (Some(summary), Some(at)) = (&workspace.summary, &workspace.summary_at) {
        if !summary.is_empty() {
            return Ok(WorkspaceSummaryResult {
                workspace_id,
                summary: summary.clone(),
                summary_at: at.clone(),
                summary_source: workspace.summary_source.clone(),
                from_llm: true,
            });
        }
    }

    let threads: Vec<_> = snapshot
        .threads
        .iter()
        .filter(|thread| thread.workspace_id == workspace_id)
        .cloned()
        .collect();

    let prompt = summaries::build_summary_prompt(&workspace, &threads);
    let (summary, from_llm) = match state.llm.summarise(&workspace_id, &prompt).await {
        Ok(Some(result)) if !result.text.is_empty() => (result.text, true),
        _ => (
            summaries::fallback_summary(&workspace, &threads),
            false,
        ),
    };

    state
        .store
        .set_workspace_summary(&workspace_id, &summary, SUMMARY_MODEL_LABEL)?;

    let updated = state
        .store
        .workspace_by_id(&workspace_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;

    Ok(WorkspaceSummaryResult {
        workspace_id,
        summary,
        summary_at: updated
            .summary_at
            .unwrap_or_else(|| "now".into()),
        summary_source: updated.summary_source,
        from_llm,
    })
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

pub fn emit_workspace_status(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
    status: crate::workspace::WorkspaceLifecycle,
) {
    let version = state.bump_event_version();
    let _ = WorkspaceStatusChanged {
        version,
        workspace_id: workspace_id.to_string(),
        status,
    }
    .emit(app);
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

fn process_trees(state: &AppState) -> Vec<(String, Vec<u32>)> {
    let mut trees: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();
    for (id, pid) in state.process.pids() {
        trees.entry(id).or_default().push(pid);
    }
    trees.into_iter().collect()
}
