//! What the webview asks of a thread: prompts, stops, sessions, and the model it runs on.

use tauri::State;

use crate::attachments::PromptAttachment;
use crate::engines::{ModelOption, ThreadModels};
use crate::error::Result;
use crate::state::AppState;

use super::{ResumeThreadSessionInput, SentMessage, thread_row};

#[tauri::command]
#[specta::specta]
pub async fn send_thread_prompt(
    thread_id: String,
    text: String,
    attachments: Vec<PromptAttachment>,
    state: State<'_, AppState>,
) -> Result<Option<SentMessage>> {
    super::submit(&state, &thread_id, text, attachments).await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_thread_turn(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    super::stop(&state, &thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn pause_thread(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    super::pause(&state, &thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_thread(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    super::resume(&state, &thread_id).await
}

/// Like `/clear`: the agent forgets the conversation and the thread starts a fresh one.
#[tauri::command]
#[specta::specta]
pub async fn new_thread_session(thread_id: String, state: State<'_, AppState>) -> Result<()> {
    super::new_session(&state, &thread_id).await
}

/// Picks an earlier conversation in the thread back up: the agent resumes the session it
/// ended with, and the thread shows that conversation again.
#[tauri::command]
#[specta::specta]
pub async fn resume_thread_session(
    input: ResumeThreadSessionInput,
    state: State<'_, AppState>,
) -> Result<()> {
    super::resume_session(&state, input).await
}

/// The models the thread's composer can pick from, and the current pick.
#[tauri::command]
#[specta::specta]
pub async fn thread_models(thread_id: String, state: State<'_, AppState>) -> Result<ThreadModels> {
    thread_row(&state, &thread_id)?;
    state.engines.thread_models(&thread_id)
}

/// The models a new thread can start on.
#[tauri::command]
#[specta::specta]
pub async fn engine_models() -> Result<Vec<ModelOption>> {
    Ok(crate::engines::static_models())
}

/// `None` goes back to the engine's default. A live engine switches before its next turn.
#[tauri::command]
#[specta::specta]
pub async fn set_thread_model(
    thread_id: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    thread_row(&state, &thread_id)?;
    state.engines.set_thread_model(&thread_id, model)
}

/// The effort levels the thread's composer can pick from, and the current pick.
#[tauri::command]
#[specta::specta]
pub async fn thread_efforts(thread_id: String, state: State<'_, AppState>) -> Result<ThreadModels> {
    thread_row(&state, &thread_id)?;
    state.engines.thread_efforts(&thread_id)
}

/// The effort levels a new thread can start on.
#[tauri::command]
#[specta::specta]
pub async fn engine_efforts() -> Result<Vec<ModelOption>> {
    Ok(crate::engines::static_efforts())
}

/// `None` goes back to the engine's default. A live engine switches before its next turn.
#[tauri::command]
#[specta::specta]
pub async fn set_thread_effort(
    thread_id: String,
    effort: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    thread_row(&state, &thread_id)?;
    state.engines.set_thread_effort(&thread_id, effort)
}
