use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::composer::{persist_agent_event, persist_user_message};
use crate::engines::{AgentEvent, ToolCallStatus, ToolKind};
use crate::error::{Error, Result};
use crate::feedback::emit_toast;
use crate::ipc::events::StateChanged;
use crate::ipc::types::{StateChangeKind, ToastPart, ToastRaisedPayload, ToastTone};
use crate::state::AppState;
use crate::store::Store;
use crate::store::types::FindingRow;
use crate::workspace::ThreadActivity;
use tauri_specta::Event;

pub const SEVERITIES: &[&str] = &["blocking", "suggestion", "nit"];

pub fn validate_severity(severity: &str) -> Result<()> {
    if SEVERITIES.contains(&severity) {
        Ok(())
    } else {
        Err(Error::Mcp(
            "severity must be blocking, suggestion, or nit".into(),
        ))
    }
}

pub fn validate_finding_location(
    worktree: &Path,
    file: Option<&str>,
    line: Option<i64>,
) -> Result<()> {
    let Some(path) = file.filter(|value| !value.trim().is_empty()) else {
        if line.is_some() {
            return Err(Error::Mcp("line requires a file path".into()));
        }
        return Ok(());
    };
    let relative = path.trim_start_matches("./");
    if relative.contains("..") {
        return Err(Error::Mcp("file path must stay inside the worktree".into()));
    }
    let full = worktree.join(relative);
    if !full.is_file() {
        return Err(Error::Mcp(format!("file not found in worktree: {path}")));
    }
    if let Some(line_no) = line {
        if line_no < 1 {
            return Err(Error::Mcp("line must be a positive integer".into()));
        }
        let contents = std::fs::read_to_string(&full)
            .map_err(|error| Error::Mcp(format!("could not read {path}: {error}")))?;
        let max_line = contents.lines().count() as i64;
        if line_no > max_line {
            return Err(Error::Mcp(format!(
                "line {line_no} is out of range for {path} ({max_line} lines)"
            )));
        }
    }
    Ok(())
}

pub fn worktree_for_workspace(store: &Store, workspace_id: &str) -> Result<PathBuf> {
    let snapshot = store.snapshot()?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|row| row.id == workspace_id)
        .ok_or_else(|| Error::Workspace(format!("unknown workspace {workspace_id}")))?;
    Ok(PathBuf::from(&workspace.worktree_path))
}

pub fn findings_batch_key(thread_id: &str) -> String {
    format!("findings-batch:{thread_id}")
}

pub async fn send_workspace_findings(
    app: &AppHandle,
    workspace_id: &str,
    thread_id: &str,
    finding_ids: Vec<String>,
) -> Result<()> {
    if finding_ids.is_empty() {
        return Err(Error::Workspace("no findings selected".into()));
    }
    let state = app.state::<AppState>();
    let snapshot = state.store.snapshot()?;
    let thread = snapshot
        .threads
        .iter()
        .find(|row| row.id == thread_id)
        .cloned()
        .ok_or_else(|| Error::Workspace(format!("unknown thread {thread_id}")))?;
    if thread.workspace_id != workspace_id {
        return Err(Error::Workspace(
            "thread does not belong to this workspace".into(),
        ));
    }

    let mut picked: Vec<FindingRow> = Vec::new();
    for id in &finding_ids {
        let Some(row) = state.store.finding_by_id(id)? else {
            return Err(Error::Store(format!("unknown finding {id}")));
        };
        if row.workspace_id != workspace_id {
            return Err(Error::Store(format!(
                "finding {id} is in another workspace"
            )));
        }
        if row.status != "open" {
            return Err(Error::Store(format!("finding {id} is not open")));
        }
        picked.push(row);
    }

    state.store.mark_findings_sent(&finding_ids, thread_id)?;
    state.store.set_setting(
        &findings_batch_key(thread_id),
        &serde_json::to_string(&finding_ids).map_err(|error| Error::Store(error.to_string()))?,
    )?;

    let count = picked.len();
    let count_label = if count == 1 {
        "1 review finding".into()
    } else {
        format!("{count} review findings")
    };
    let user_text = format!(
        "Fix the {count_label} I selected. Keep each fix small, add a test where the finding asks for one, and don’t push yet.\n\nFinding ids: {}",
        finding_ids.join(", ")
    );
    persist_user_message(&state.store, thread_id, &user_text)?;

    let titles = picked
        .iter()
        .map(|row| row.title.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    persist_tool_step(
        &state.store,
        thread_id,
        &format!("Received {count} findings from the review"),
        Some(&titles),
    )?;

    let live_title = picked
        .first()
        .map(|row| {
            let file = row.file.as_deref().unwrap_or("unknown file");
            format!("{} · {file}", row.title)
        })
        .unwrap_or_else(|| format!("Fixing {count} findings…"));
    persist_current_tool(&state.store, thread_id, &live_title)?;

    state.store.set_thread_status(thread_id, "running")?;
    state
        .workspace
        .set_thread(thread_id, workspace_id, ThreadActivity::Running)
        .await;

    let engine_prompt = build_fix_prompt(&picked);
    let _ = state.engines.submit_prompt(thread_id, engine_prompt, false);

    emit_toast(
        app,
        ToastRaisedPayload {
            tone: ToastTone::Ok,
            parts: vec![
                ToastPart::Text {
                    value: format!("Sent {count} findings to "),
                },
                ToastPart::Text {
                    value: thread.title.clone(),
                },
            ],
            workspace_id: Some(workspace_id.to_string()),
        },
    );

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);

    Ok(())
}

fn build_fix_prompt(findings: &[FindingRow]) -> String {
    let mut lines = vec![
        "Work through these review findings one at a time.".into(),
        "After each fix, call the Harness MCP tool `mark_finding_fixed` with the finding id and commit sha."
            .into(),
    ];
    for row in findings {
        let loc = match (&row.file, row.line) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file.clone(),
            _ => "no file".into(),
        };
        lines.push(format!(
            "- id={} [{}] {} ({}) — {}",
            row.id, row.severity, row.title, loc, row.explanation
        ));
    }
    lines.join("\n")
}

pub async fn on_finding_marked_fixed(app: &AppHandle, finding_id: &str) -> Result<()> {
    let state = app.state::<AppState>();
    let Some(finding) = state.store.finding_by_id(finding_id)? else {
        return Ok(());
    };
    let Some(thread_id) = finding.sent_to_thread_id.clone() else {
        return Ok(());
    };
    let batch_raw = state
        .store
        .get_setting(&findings_batch_key(&thread_id))?
        .unwrap_or_default();
    let batch: Vec<String> = serde_json::from_str(&batch_raw).unwrap_or_default();
    if batch.is_empty() || !batch.contains(&finding.id) {
        return Ok(());
    }
    let snapshot = state.store.snapshot()?;
    let mut batch_rows = Vec::new();
    for id in &batch {
        if let Some(row) = snapshot.findings.iter().find(|item| &item.id == id) {
            batch_rows.push(row.clone());
        }
    }
    if !batch_rows.iter().all(|row| row.status == "fixed") {
        return Ok(());
    }

    let count = batch_rows.len();
    let files: Vec<String> = batch_rows
        .iter()
        .filter_map(|row| row.file.as_ref())
        .map(|path| {
            Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path.as_str())
                .to_string()
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let detail = files.join(", ");
    persist_tool_step(
        &state.store,
        &thread_id,
        &format!("Fixed {count} findings"),
        Some(&detail),
    )?;

    state.store.set_thread_status(&thread_id, "idle")?;
    let workspace_id = snapshot
        .threads
        .iter()
        .find(|row| row.id == thread_id)
        .map(|row| row.workspace_id.clone())
        .unwrap_or_default();
    if !workspace_id.is_empty() {
        state
            .workspace
            .set_thread(&thread_id, &workspace_id, ThreadActivity::Idle)
            .await;
    }
    state
        .store
        .set_setting(&findings_batch_key(&thread_id), "[]")?;

    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
    Ok(())
}

pub fn persist_tool_step(
    store: &Store,
    thread_id: &str,
    title: &str,
    detail: Option<&str>,
) -> Result<()> {
    let event = AgentEvent::ToolCall {
        id: format!("finding-{}", Uuid::new_v4()),
        title: title.to_string(),
        name: None,
        kind: ToolKind::Other,
        status: ToolCallStatus::Completed,
        locations: Vec::new(),
        detail: detail.map(str::to_string),
    };
    persist_agent_event(store, thread_id, &event)
}

pub fn persist_current_tool(store: &Store, thread_id: &str, title: &str) -> Result<()> {
    let event = AgentEvent::CurrentTool {
        id: None,
        title: title.to_string(),
    };
    persist_agent_event(store, thread_id, &event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn validates_file_and_line_in_worktree() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("src/a.ts");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "line1\nline2\n").unwrap();
        validate_finding_location(dir.path(), Some("src/a.ts"), Some(2)).unwrap();
        assert!(validate_finding_location(dir.path(), Some("src/a.ts"), Some(9)).is_err());
        assert!(validate_finding_location(dir.path(), Some("../escape"), None).is_err());
    }
}
