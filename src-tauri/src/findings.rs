use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::engines::AgentEvent;
use crate::error::{Error, Result};
use crate::feedback::{Change, ToastPart, ToastRaisedPayload, ToastTone};
use crate::state::AppState;
use crate::store::types::FindingRow;
use crate::timeline::TimelineStep;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SendWorkspaceFindingsInput {
    pub workspace_id: String,
    pub thread_id: String,
    pub finding_ids: Vec<String>,
}

/// Findings to dismiss, or (with `dismissed: false`) to restore.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DismissWorkspaceFindingsInput {
    pub workspace_id: String,
    pub finding_ids: Vec<String>,
    pub dismissed: bool,
}

pub fn findings_batch_key(thread_id: &str) -> String {
    format!("findings-batch:{thread_id}")
}

pub async fn send_workspace_findings(
    state: &AppState,
    workspace_id: &str,
    thread_id: &str,
    mut finding_ids: Vec<String>,
) -> Result<()> {
    // A repeated id would pass the "open" check twice, then fail the mark after the prompt
    // had already gone out.
    let mut seen = std::collections::HashSet::new();
    finding_ids.retain(|id| seen.insert(id.clone()));
    if finding_ids.is_empty() {
        return Err(Error::Workspace("no findings selected".into()));
    }
    let thread = state
        .store
        .thread_by_id(thread_id)?
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
    // First, so findings that never reached the thread stay open to send again.
    crate::turn::start(state, &thread, &user_text, build_fix_prompt(&picked)).await?;
    state.store.mark_findings_sent(&finding_ids, thread_id)?;
    state.store.set_setting(
        &findings_batch_key(thread_id),
        &serde_json::to_string(&finding_ids).map_err(|error| Error::Store(error.to_string()))?,
    )?;

    let titles = picked.iter().map(|row| row.title.clone()).collect();
    state.transcript.record_step(
        thread_id,
        TimelineStep::FindingsReceived {
            count: count as u32,
            titles,
        },
    )?;

    let live_title = picked
        .first()
        .map(|row| {
            let file = row.file.as_deref().unwrap_or("unknown file");
            format!("{} · {file}", row.title)
        })
        .unwrap_or_else(|| format!("Fixing {count} findings…"));
    // Saved, unlike an engine's: it names the batch for as long as the thread works on it.
    state.transcript.append(
        thread_id,
        AgentEvent::CurrentTool {
            id: None,
            title: live_title,
        },
    )?;

    state.out.toast(ToastRaisedPayload {
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
        thread_id: Some(thread_id.to_string()),
    });

    state.out.changed(Change::THREADS);

    Ok(())
}

/// Dismissed findings stay listed but aren't sent to an agent or posted with the review.
pub fn dismiss_findings(state: &AppState, input: DismissWorkspaceFindingsInput) -> Result<()> {
    state
        .store
        .set_findings_dismissed(&input.workspace_id, &input.finding_ids, input.dismissed)?;
    state.out.changed(Change::THREADS);
    Ok(())
}

fn build_fix_prompt(findings: &[FindingRow]) -> String {
    let mut lines = vec![
        "Work through these review findings one at a time.".into(),
        "After each fix, call the Cormux MCP tool `mark_finding_fixed` with the finding id and commit sha."
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

/// What the `mark_finding_fixed` tool the fix prompt names will do. The tool isn't offered to
/// engines yet, so only tests call this.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tool is offered in a later change")
)]
pub async fn mark_fixed(state: &AppState, finding_id: &str, commit_sha: &str) -> Result<()> {
    state.store.mark_finding_fixed(finding_id, commit_sha)?;
    on_finding_marked_fixed(state, finding_id).await
}

/// Once every finding in the thread's batch is fixed, records it and idles the thread.
async fn on_finding_marked_fixed(state: &AppState, finding_id: &str) -> Result<()> {
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
    let mut batch_rows = Vec::with_capacity(batch.len());
    for id in &batch {
        if let Some(row) = state.store.finding_by_id(id)? {
            batch_rows.push(row);
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
    state.transcript.record_step(
        &thread_id,
        TimelineStep::FindingsFixed {
            count: count as u32,
            files,
        },
    )?;

    if let Some(thread) = state.store.thread_by_id(&thread_id)? {
        crate::turn::mark_idle(state, &thread).await?;
    }
    state
        .store
        .set_setting(&findings_batch_key(&thread_id), "[]")?;

    state.out.changed(Change::THREADS);
    Ok(())
}
