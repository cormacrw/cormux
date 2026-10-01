use serde::{Deserialize, Serialize};
use specta::Type;

use super::events::EngineKind;

/// One entry in the composer's model picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    /// What the engine is told: a `--model` alias for Claude, an ACP config value otherwise.
    pub id: String,
    pub label: String,
}

/// The models a thread can switch to, and the one it's on (`None` is the engine's default).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadModels {
    pub current: Option<String>,
    pub options: Vec<ModelOption>,
}

/// Claude Code takes these aliases for `--model` and resolves them to the latest of each family.
/// ACP engines list their own models once a session starts.
pub fn static_models(kind: EngineKind) -> Vec<ModelOption> {
    match kind {
        EngineKind::Claude => [("opus", "Opus"), ("sonnet", "Sonnet"), ("haiku", "Haiku")]
            .into_iter()
            .map(|(id, label)| ModelOption {
                id: id.into(),
                label: label.into(),
            })
            .collect(),
        EngineKind::Cursor | EngineKind::Codex | EngineKind::Gemini => Vec::new(),
    }
}
