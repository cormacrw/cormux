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
    record_batch(state, thread_id, &finding_ids)?;

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

/// The findings a thread was sent to fix, which it works through as one batch.
pub fn record_batch(state: &AppState, thread_id: &str, finding_ids: &[String]) -> Result<()> {
    state.store.mark_findings_sent(finding_ids, thread_id)?;
    state.store.set_setting(
        &findings_batch_key(thread_id),
        &serde_json::to_string(finding_ids).map_err(|error| Error::Store(error.to_string()))?,
    )
}

fn open_batch(state: &AppState, thread_id: &str) -> Result<Vec<String>> {
    let raw = state
        .store
        .get_setting(&findings_batch_key(thread_id))?
        .unwrap_or_default();
    Ok(serde_json::from_str(&raw).unwrap_or_default())
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
        "After each fix, commit it and call the cormux tool mark_finding_fixed with the finding id and the commit's sha."
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

/// A finding in `thread_id`'s open batch is fixed. Once every finding in the batch is, the batch
/// is recorded as done and the thread idles.
pub async fn mark_fixed(
    state: &AppState,
    thread_id: &str,
    finding_id: &str,
    commit_sha: &str,
) -> Result<()> {
    let sent_here = state
        .store
        .finding_by_id(finding_id)?
        .is_some_and(|finding| finding.sent_to_thread_id.as_deref() == Some(thread_id));
    let batch = open_batch(state, thread_id)?;
    if !sent_here || !batch.iter().any(|id| id == finding_id) {
        return Err(Error::Workspace(format!(
            "finding {finding_id} isn't one this thread was sent to fix"
        )));
    }
    state.store.mark_finding_fixed(finding_id, commit_sha)?;
    finish_batch_if_fixed(state, thread_id, &batch).await
}

async fn finish_batch_if_fixed(state: &AppState, thread_id: &str, batch: &[String]) -> Result<()> {
    let mut batch_rows = Vec::with_capacity(batch.len());
    for id in batch {
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
        thread_id,
        TimelineStep::FindingsFixed {
            count: count as u32,
            files,
        },
    )?;

    if let Some(thread) = state.store.thread_by_id(thread_id)? {
        crate::turn::mark_idle(state, &thread).await?;
    }
    state
        .store
        .set_setting(&findings_batch_key(thread_id), "[]")?;

    state.out.changed(Change::THREADS);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::EngineCommand;
    use crate::feedback::RecordingOutput;
    use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle};
    use crate::store::types::{RepoRecord, ThreadRow, WorkspaceRow};
    use crate::thread_runtime::fake::FakeEngines;
    use std::sync::Arc;
    use tokio::sync::mpsc::UnboundedReceiver;

    const T: &str = "t1";
    const W: &str = "w1";

    async fn with_findings(
        status: ThreadStatus,
    ) -> (AppState, FakeEngines, Vec<String>, Arc<RecordingOutput>) {
        let (state, recording, fake) =
            AppState::faked(crate::paths::DataDir::new(std::env::temp_dir()));
        state
            .store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: None,
                single_instance: false,
            })
            .unwrap();
        state
            .store
            .insert_workspace(
                &WorkspaceRow::new(
                    WorkspaceKind::Lead,
                    W,
                    "r1",
                    "Work",
                    "feat/x",
                    "main",
                    std::env::temp_dir().display().to_string(),
                )
                .with_status(WorkspaceLifecycle::Idle),
            )
            .unwrap();
        state
            .store
            .upsert_thread(&ThreadRow::new(T, W, ThreadRole::Lead, "claude").with_status(status))
            .unwrap();
        let ids = vec!["f1".to_string(), "f2".to_string()];
        for id in &ids {
            state
                .store
                .upsert_finding(&FindingRow {
                    id: id.clone(),
                    workspace_id: W.into(),
                    severity: "blocking".into(),
                    title: format!("Finding {id}"),
                    file: None,
                    line: None,
                    explanation: "wrong".into(),
                    status: "open".into(),
                    commit_sha: None,
                    sent_to_thread_id: None,
                })
                .unwrap();
        }
        (state, fake, ids, recording)
    }

    fn finding_status(state: &AppState, id: &str) -> String {
        state.store.finding_by_id(id).unwrap().unwrap().status
    }

    fn thread_status(state: &AppState) -> String {
        state.store.thread_by_id(T).unwrap().unwrap().status
    }

    fn stored_workspace(state: &AppState) -> String {
        state.store.workspace_by_id(W).unwrap().unwrap().status
    }

    fn prompts(engine: &mut UnboundedReceiver<EngineCommand>) -> Vec<String> {
        std::iter::from_fn(|| engine.try_recv().ok())
            .filter_map(|command| match command {
                EngineCommand::Prompt(prompt) => Some(prompt.text),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn findings_that_cant_be_sent_say_so_and_can_be_sent_again() {
        let (state, fake, ids, recording) = with_findings(ThreadStatus::Idle).await;
        fake.refuse_sends(T);

        let sent = send_workspace_findings(&state, W, T, ids).await;

        assert!(sent.is_err(), "the failure reaches the caller");
        assert_eq!(thread_status(&state), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(finding_status(&state, "f1"), "open");
        assert!(recording.toasts().is_empty(), "nothing claims they went");
    }

    #[tokio::test]
    async fn sent_findings_start_a_turn_that_names_the_tool_to_close_them() {
        let (state, fake, ids, recording) = with_findings(ThreadStatus::Idle).await;
        let mut engine = fake.engine(T);

        send_workspace_findings(&state, W, T, ids).await.unwrap();

        let sent = prompts(&mut engine);
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("mark_finding_fixed"));
        assert_eq!(thread_status(&state), "running");
        assert_eq!(stored_workspace(&state), "running");
        assert_eq!(finding_status(&state, "f1"), "sent");
        assert_eq!(recording.toasts().len(), 1);
    }

    #[tokio::test]
    async fn findings_sent_to_a_paused_thread_wait_for_its_resume() {
        let (state, fake, ids, _recording) = with_findings(ThreadStatus::Paused).await;
        let mut engine = fake.engine(T);

        send_workspace_findings(&state, W, T, ids).await.unwrap();

        assert_eq!(thread_status(&state), "paused");
        assert!(prompts(&mut engine).is_empty());

        crate::turn::resume(&state, T).await.unwrap();
        assert!(prompts(&mut engine)[0].contains("id=f1"));
    }

    #[tokio::test]
    async fn the_last_finding_fixed_idles_the_workspace_too() {
        let (state, _fake, ids, _recording) = with_findings(ThreadStatus::Idle).await;
        send_workspace_findings(&state, W, T, ids).await.unwrap();
        assert_eq!(stored_workspace(&state), "running");

        mark_fixed(&state, T, "f1", "abc").await.unwrap();
        assert_eq!(thread_status(&state), "running");
        mark_fixed(&state, T, "f2", "abc").await.unwrap();

        assert_eq!(thread_status(&state), "idle");
        assert_eq!(stored_workspace(&state), "idle");
        assert_eq!(
            state.workspaces.get(W).await.unwrap().status,
            WorkspaceLifecycle::Idle
        );
    }

    #[tokio::test]
    async fn a_finding_outside_the_threads_batch_cant_be_marked() {
        let (state, _fake, ids, _recording) = with_findings(ThreadStatus::Idle).await;
        send_workspace_findings(&state, W, T, vec![ids[0].clone()])
            .await
            .unwrap();

        assert!(mark_fixed(&state, T, "f2", "abc").await.is_err());
        assert!(mark_fixed(&state, "other", "f1", "abc").await.is_err());
        assert_eq!(finding_status(&state, "f2"), "open");
        assert_eq!(finding_status(&state, "f1"), "sent");
    }
}
