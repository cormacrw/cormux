//! What the webview reads of a thread's transcript.

use tauri::State;

use crate::error::Result;
use crate::state::AppState;

use super::{ThreadConversations, TranscriptPage};

/// A window of a thread's transcript: its newest rows by default, those after `after_seq` to
/// catch up, or those before `before_seq` to scroll back.
#[tauri::command]
#[specta::specta]
pub async fn get_thread_events(
    thread_id: String,
    after_seq: Option<i64>,
    before_seq: Option<i64>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<TranscriptPage> {
    let transcript = state.transcript.clone();
    tokio::task::spawn_blocking(move || transcript.window(&thread_id, after_seq, before_seq, limit))
        .await?
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_conversations(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<ThreadConversations> {
    let transcript = state.transcript.clone();
    tokio::task::spawn_blocking(move || transcript.conversations(&thread_id)).await?
}
