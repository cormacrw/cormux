//! Scratches: a titled, one-off conversation against a repo's main checkout. No worktree, no
//! branch, no setup commands. The engine runs in its own read-only mode (see `SpawnSpec::read_only`).

use std::path::PathBuf;

use tauri::AppHandle;
use tauri_specta::Event;
use uuid::Uuid;

use crate::composer::{persist_agent_event, persist_user_message};
use crate::engines::{AgentEvent, EngineKind, SpawnSpec, ToolCallStatus, ToolKind};
use crate::error::{Error, Result};
use crate::ipc::commands::expand_tilde;
use crate::ipc::events::StateChanged;
use crate::ipc::types::{
    CreateScratchInput, CreateScratchResult, StateChangeKind, ToastPart, ToastTone,
};
use crate::state::AppState;
use crate::store::types::{ScratchRow, ThreadRow};

pub const TITLE_MAX: usize = 64;

pub async fn create(
    app: &AppHandle,
    state: &AppState,
    input: CreateScratchInput,
) -> Result<CreateScratchResult> {
    let repo = state
        .store
        .snapshot()?
        .repos
        .into_iter()
        .find(|row| row.id == input.repo_id)
        .ok_or_else(|| Error::Workspace(format!("unknown repo {}", input.repo_id)))?;
    let prompt = input
        .prompt
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    // A blank title drafts one from the prompt now and asks Haiku for a better one below.
    let given: String = input.title.trim().chars().take(TITLE_MAX).collect();
    let auto_name_from = if given.is_empty() { prompt.clone() } else { None };
    let title = if !given.is_empty() {
        given
    } else if let Some(text) = &prompt {
        crate::naming::draft_title(text, TITLE_MAX)
    } else {
        crate::naming::UNTITLED_SCRATCH.to_string()
    };

    let scratch_id = format!("scratch-{}", Uuid::new_v4().simple());
    let thread_id = Uuid::new_v4().to_string();
    state.store.upsert_thread(&ThreadRow {
        id: thread_id.clone(),
        workspace_id: scratch_id.clone(),
        title: "Scratch".into(),
        engine: default_engine(state),
        session_id: None,
        status: if prompt.is_some() { "running" } else { "idle" }.into(),
        used_tokens: None,
        context_size: None,
        cost_usd: None,
        transcript_readonly: false,
    })?;
    state.store.insert_scratch(&ScratchRow {
        id: scratch_id.clone(),
        repo_id: repo.id.clone(),
        title: title.clone(),
        thread_id: thread_id.clone(),
        engine: String::new(),
        status: String::new(),
        created_at: String::new(),
    })?;

    if let Some(text) = prompt {
        persist_user_message(&state.store, &thread_id, &text)?;
        persist_agent_event(
            &state.store,
            &thread_id,
            &AgentEvent::ToolCall {
                id: format!("scratch-open-{}", Uuid::new_v4()),
                title: format!("Opened {}", repo.id),
                name: None,
                kind: ToolKind::Read,
                status: ToolCallStatus::Completed,
                locations: Vec::new(),
                detail: Some(repo.path.clone()),
            },
        )?;
        // Starting the engine waits on its handshake, so the page opens first.
        let app = app.clone();
        let thread_id = thread_id.clone();
        tauri::async_runtime::spawn(async move {
            use tauri::Manager;
            let state = app.state::<AppState>();
            let started = match state.store.scratch_for_thread(&thread_id) {
                Ok(Some(scratch)) => ensure_engine(&state, &scratch).await,
                Ok(None) => return,
                Err(error) => Err(error),
            };
            let sent = started.and_then(|()| state.engines.prompt(&thread_id, text));
            if let Err(error) = sent {
                fail_turn(&state, &thread_id, &error);
                crate::feedback::emit_toast_parts(
                    &app,
                    ToastTone::Bad,
                    vec![ToastPart::Text {
                        value: format!("Could not start the agent: {error}"),
                    }],
                    None,
                );
            }
            emit_changed(&app, &state);
        });
    }

    if let Some(text) = auto_name_from {
        spawn_auto_title(app.clone(), scratch_id.clone(), title, text);
    }

    emit_changed(app, state);
    Ok(CreateScratchResult {
        scratch_id,
        thread_id,
    })
}

/// A follow-up from the scratch composer. Mirrors `send_thread_prompt` without the workspace.
pub async fn send(
    app: &AppHandle,
    state: &AppState,
    scratch: &ScratchRow,
    text: String,
) -> Result<()> {
    let held = scratch.status == "paused";
    if !held {
        ensure_engine(state, scratch).await?;
    }
    persist_user_message(&state.store, &scratch.thread_id, &text)?;
    state.engines.submit_prompt(&scratch.thread_id, text, held)?;
    if !held {
        state.store.set_thread_status(&scratch.thread_id, "running")?;
    }
    emit_changed(app, state);
    Ok(())
}

/// End and Delete are the same operation: stop the engine and discard the conversation.
/// The repo is never touched.
pub async fn end(app: &AppHandle, state: &AppState, scratch_id: &str) -> Result<()> {
    let scratch = state
        .store
        .scratch_by_id(scratch_id)?
        .ok_or_else(|| Error::Workspace(format!("unknown scratch {scratch_id}")))?;
    for approval in state.store.pending_approvals_for_thread(&scratch.thread_id)? {
        let _ = state
            .approvals
            .resolve(&approval.id, false, Some("The scratch was ended".into()))
            .await;
    }
    state.store.delete_scratch(scratch_id)?;
    emit_changed(app, state);
    // Shutdown waits out a grace period, so the UI does not wait for it.
    let engines = state.engines.clone();
    let thread_id = scratch.thread_id;
    tauri::async_runtime::spawn(async move {
        let _ = engines.stop(&thread_id).await;
    });
    Ok(())
}

async fn ensure_engine(state: &AppState, scratch: &ScratchRow) -> Result<()> {
    if state.engines.has_thread(&scratch.thread_id) {
        return Ok(());
    }
    let repo = state
        .store
        .snapshot()?
        .repos
        .into_iter()
        .find(|row| row.id == scratch.repo_id)
        .ok_or_else(|| {
            Error::Workspace(format!(
                "{} was removed from Settings, so this scratch can't reach it",
                scratch.repo_id
            ))
        })?;
    let cwd: PathBuf = expand_tilde(&repo.path);
    if !cwd.is_dir() {
        return Err(Error::Workspace(format!("{} does not exist", repo.path)));
    }
    state
        .engines
        .spawn(SpawnSpec {
            thread_id: scratch.thread_id.clone(),
            kind: engine_kind(&scratch.engine),
            cwd,
            resume: None,
            override_argv: None,
            // Reading never asks; anything else follows the Run everything setting.
            auto_approve_readonly: true,
            auto_approve_all: crate::approvals::run_everything_from_store(&state.store),
            read_only: true,
        })
        .await
}

fn fail_turn(state: &AppState, thread_id: &str, error: &Error) {
    let event = AgentEvent::TurnEnd {
        stop_reason: "error".into(),
        error: Some(error.to_string()),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.store.append_event(thread_id, "turn_end", &payload);
    }
    let _ = state.store.set_thread_status(thread_id, "idle");
}

/// Swaps the drafted title for Haiku's unless the scratch was renamed or ended meanwhile.
fn spawn_auto_title(app: AppHandle, scratch_id: String, draft: String, prompt: String) {
    tauri::async_runtime::spawn(async move {
        use tauri::Manager;
        let state = app.state::<AppState>();
        let Some(title) =
            crate::naming::generate_title(&state.llm, &prompt, "scratch", TITLE_MAX).await
        else {
            return;
        };
        match state.store.scratch_by_id(&scratch_id) {
            Ok(Some(scratch)) if scratch.title == draft => {}
            _ => return,
        }
        if state.store.set_scratch_title(&scratch_id, &title).is_ok() {
            emit_changed(&app, &state);
        }
    });
}

fn emit_changed(app: &AppHandle, state: &AppState) {
    let version = state.bump_event_version();
    let _ = StateChanged {
        version,
        kind: StateChangeKind::WorkspaceStatus,
    }
    .emit(app);
}

fn default_engine(state: &AppState) -> String {
    state
        .store
        .get_setting("defaultEngine")
        .ok()
        .flatten()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "claude".into())
}

fn engine_kind(engine: &str) -> EngineKind {
    EngineKind::all()
        .iter()
        .copied()
        .find(|kind| kind.as_str().eq_ignore_ascii_case(engine))
        .unwrap_or(EngineKind::Claude)
}
