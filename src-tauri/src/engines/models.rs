use serde::{Deserialize, Serialize};
use specta::Type;

use super::events::EngineKind;

/// One entry in the composer's model or effort picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    /// What the engine is told: a `--model` alias or `--effort` level for Claude.
    pub id: String,
    pub label: String,
}

/// The models or effort levels a thread can switch to, and the one it's on (`None` is the
/// engine's default).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadModels {
    pub current: Option<String>,
    pub options: Vec<ModelOption>,
}

/// Claude Code takes these aliases for `--model` and resolves them to the latest of each family.
pub fn static_models(kind: EngineKind) -> Vec<ModelOption> {
    match kind {
        EngineKind::Claude => [
            ("fable", "Fable"),
            ("opus", "Opus"),
            ("sonnet", "Sonnet"),
            ("haiku", "Haiku"),
        ]
        .into_iter()
        .map(|(id, label)| ModelOption {
            id: id.into(),
            label: label.into(),
        })
        .collect(),
    }
}

/// Claude Code's `--effort` levels. It settles a level a model can't do on the nearest it can.
pub fn static_efforts(kind: EngineKind) -> Vec<ModelOption> {
    match kind {
        EngineKind::Claude => [
            ("low", "Low"),
            ("medium", "Medium"),
            ("high", "High"),
            ("xhigh", "Extra high"),
            ("max", "Max"),
        ]
        .into_iter()
        .map(|(id, label)| ModelOption {
            id: id.into(),
            label: label.into(),
        })
        .collect(),
    }
}
