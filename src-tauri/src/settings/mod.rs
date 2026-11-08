//! The user's app-wide preferences: one typed struct, stored one row per field in the settings
//! table under the field's wire name, so the struct is the only list of keys (and, through its
//! `Default`, of defaults: the webview gets them as `DEFAULT_SETTINGS`).

pub mod commands;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

use crate::engines::EngineKind;
use crate::error::Result;
use crate::store::Store;

/// Scratch macros, which bangs replaced; read while no bangs are saved.
const LEGACY_MACROS: &str = "scratchMacros";
const BANGS: &str = "bangs";

/// Where new worktrees go when the user hasn't picked a folder.
pub const DEFAULT_WORKTREE_ROOT: &str = "~/.harness/worktrees";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub reduce_motion: bool,
    /// Approve every tool request without asking. Questions still wait for the user.
    pub run_everything: bool,
    /// The experimental References panel, and the tool that pins links to it.
    pub experimental_references: bool,
    pub notify_approvals: bool,
    pub notify_review_finished: bool,
    pub default_engine: EngineKind,
    /// Preferred repo id for new threads; it may have been removed since.
    pub default_repo: String,
    /// The folder new worktrees go in, as the user typed it (`~` allowed).
    pub worktree_root: String,
    /// The app Open in Terminal launches; empty means Terminal.
    pub terminal_app: String,
    /// Instructions for drafting a PR description; empty means the built-in default.
    pub pr_prompt: String,
    /// `!name` prompts, for any prompt bar and the command palette.
    pub bangs: Vec<Bang>,
    pub clickup_workspace_id: String,
    pub clickup_space_id: String,
    /// The sprint folder the ClickUp board shows.
    pub clickup_folder_id: String,
    /// Board lanes the user folded away, by status name.
    pub clickup_hidden_statuses: Vec<String>,
}

/// A short name like `!test` that stands for a saved prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Bang {
    pub id: String,
    /// Without the `!`: lowercase letters, digits, `-` and `_`.
    pub name: String,
    pub prompt: String,
}

/// Changes to some settings; the rest stay as they are.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    #[specta(optional)]
    pub reduce_motion: Option<bool>,
    #[specta(optional)]
    pub run_everything: Option<bool>,
    #[specta(optional)]
    pub experimental_references: Option<bool>,
    #[specta(optional)]
    pub notify_approvals: Option<bool>,
    #[specta(optional)]
    pub notify_review_finished: Option<bool>,
    #[specta(optional)]
    pub default_engine: Option<EngineKind>,
    #[specta(optional)]
    pub default_repo: Option<String>,
    /// Blank goes back to the default folder.
    #[specta(optional)]
    pub worktree_root: Option<String>,
    #[specta(optional)]
    pub terminal_app: Option<String>,
    #[specta(optional)]
    pub pr_prompt: Option<String>,
    #[specta(optional)]
    pub bangs: Option<Vec<Bang>>,
    #[specta(optional)]
    pub clickup_workspace_id: Option<String>,
    #[specta(optional)]
    pub clickup_space_id: Option<String>,
    #[specta(optional)]
    pub clickup_folder_id: Option<String>,
    #[specta(optional)]
    pub clickup_hidden_statuses: Option<Vec<String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            reduce_motion: false,
            run_everything: false,
            experimental_references: false,
            notify_approvals: true,
            notify_review_finished: true,
            default_engine: EngineKind::Claude,
            default_repo: String::new(),
            worktree_root: DEFAULT_WORKTREE_ROOT.into(),
            terminal_app: String::new(),
            pr_prompt: String::new(),
            bangs: Vec::new(),
            clickup_workspace_id: String::new(),
            clickup_space_id: String::new(),
            clickup_folder_id: String::new(),
            clickup_hidden_statuses: Vec::new(),
        }
    }
}

impl Settings {
    /// What's stored, with anything missing or unreadable at its default. A store that can't be
    /// read at all gives the defaults, which ask before running anything.
    pub fn load(store: &Store) -> Self {
        Self::read(store).unwrap_or_else(|error| {
            log::warn!("reading settings: {error}");
            Self::default()
        })
    }

    /// What's stored, with anything missing or unreadable at its default.
    pub fn read(store: &Store) -> Result<Self> {
        let rows = store.with_read(crate::store::settings_in)?;
        Ok(Self::from_rows(rows.into_iter().collect()))
    }

    /// Each stored row over the defaults, field by field: a row that doesn't read as its
    /// field's type (a word no engine has, `maybe` for a flag) leaves the default.
    fn from_rows(rows: HashMap<String, String>) -> Self {
        let defaults = Self::default();
        let mut fields = match serde_json::to_value(&defaults) {
            Ok(Value::Object(fields)) => fields,
            _ => return defaults,
        };
        let keys: Vec<String> = fields.keys().cloned().collect();
        for key in keys {
            let Some(stored) = rows
                .get(&key)
                .and_then(|raw| stored_value(&fields[&key], raw))
            else {
                continue;
            };
            let default = fields.insert(key.clone(), stored);
            if serde_json::from_value::<Self>(Value::Object(fields.clone())).is_err()
                && let Some(default) = default
            {
                fields.insert(key, default);
            }
        }
        let mut settings: Self = serde_json::from_value(Value::Object(fields)).unwrap_or(defaults);
        settings.bangs = parse_bangs(rows.get(BANGS))
            .or_else(|| parse_bangs(rows.get(LEGACY_MACROS)))
            .unwrap_or_default();
        settings
    }

    /// Saves what `patch` changes, all or nothing.
    pub fn update(store: &Store, mut patch: SettingsPatch) -> Result<()> {
        for path in [&mut patch.worktree_root, &mut patch.terminal_app]
            .into_iter()
            .flatten()
        {
            *path = path.trim().to_string();
        }
        let fields = match serde_json::to_value(&patch) {
            Ok(Value::Object(fields)) => fields,
            _ => return Ok(()),
        };
        let rows: Vec<(String, String)> = fields
            .into_iter()
            .filter(|(_, value)| !value.is_null())
            .map(|(key, value)| (key, stored_text(value)))
            .collect();
        let rows: Vec<(&str, String)> = rows
            .iter()
            .map(|(key, text)| (key.as_str(), text.clone()))
            .collect();
        store.set_settings(&rows)
    }

    /// The default engine as threads store it.
    pub fn default_engine_name(&self) -> String {
        word(&self.default_engine)
    }

    /// Saves every field.
    #[cfg(test)]
    pub fn save(&self, store: &Store) {
        let patch = serde_json::from_value(serde_json::to_value(self).unwrap()).unwrap();
        Self::update(store, patch).unwrap();
    }
}

/// How a field is stored: strings (enum words too) as they are, flags as `true`/`false`, lists
/// as JSON.
fn stored_text(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

/// A stored row read as its field's type, which its default shows; `None` leaves the default.
/// A blank string reads as the default, which is how "back to the default folder" is saved.
fn stored_value(default: &Value, raw: &str) -> Option<Value> {
    match default {
        Value::Bool(_) => match raw.trim() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        Value::String(_) => (!raw.trim().is_empty()).then(|| Value::String(raw.to_string())),
        _ => serde_json::from_str(raw).ok(),
    }
}

/// An enum's wire word, as threads store it.
fn word(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(word)) => word,
        _ => String::new(),
    }
}

/// A saved bang list; `None` when there's none to read. Names are made bang-shaped, as the
/// scratch macros they replaced had names like `Daily standup`.
fn parse_bangs(raw: Option<&String>) -> Option<Vec<Bang>> {
    let items: Vec<serde_json::Value> = serde_json::from_str(raw?).ok()?;
    Some(
        items
            .into_iter()
            .filter_map(|item| {
                let field = |name: &str| item.get(name).and_then(|value| value.as_str());
                let id = field("id").filter(|id| !id.is_empty())?;
                Some(Bang {
                    id: id.into(),
                    name: bang_name(field("name").unwrap_or("")),
                    prompt: field("prompt").unwrap_or("").into(),
                })
            })
            .collect(),
    )
}

/// A name as a bang: no `!`, no spaces (they become `-`), lowercase.
fn bang_name(raw: &str) -> String {
    let mut name = String::new();
    let mut in_space = false;
    for c in raw.trim().trim_start_matches('!').to_lowercase().chars() {
        if c.is_whitespace() {
            if !in_space {
                name.push('-');
            }
            in_space = true;
            continue;
        }
        in_space = false;
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' {
            name.push(c);
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_stored_reads_as_the_defaults() {
        let store = Store::in_memory().unwrap();
        assert_eq!(Settings::load(&store), Settings::default());
        assert!(Settings::default().notify_approvals);
        assert_eq!(Settings::default().worktree_root, DEFAULT_WORKTREE_ROOT);
    }

    #[test]
    fn an_update_changes_only_what_it_names() {
        let store = Store::in_memory().unwrap();
        Settings::update(
            &store,
            SettingsPatch {
                reduce_motion: Some(true),
                notify_approvals: Some(false),
                worktree_root: Some("  ~/trees ".into()),
                bangs: Some(vec![Bang {
                    id: "b1".into(),
                    name: "test".into(),
                    prompt: "Run the tests".into(),
                }]),
                clickup_hidden_statuses: Some(vec!["done".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        Settings::update(
            &store,
            SettingsPatch {
                worktree_root: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();

        let settings = Settings::load(&store);
        assert!(settings.reduce_motion);
        assert!(!settings.notify_approvals);
        assert!(settings.notify_review_finished);
        assert_eq!(settings.worktree_root, DEFAULT_WORKTREE_ROOT);
        assert_eq!(settings.bangs[0].name, "test");
        assert_eq!(settings.clickup_hidden_statuses, ["done"]);
        assert_eq!(settings.default_engine_name(), "claude");
    }

    #[test]
    fn rows_are_stored_under_the_wire_names_in_the_form_older_builds_wrote() {
        let store = Store::in_memory().unwrap();
        let settings = Settings {
            run_everything: true,
            terminal_app: " iTerm ".into(),
            pr_prompt: "  Keep it short".into(),
            clickup_hidden_statuses: vec!["done".into()],
            ..Settings::default()
        };
        settings.save(&store);
        let rows: HashMap<_, _> = store
            .with_read(crate::store::settings_in)
            .unwrap()
            .into_iter()
            .collect();
        assert_eq!(rows["runEverything"], "true");
        assert_eq!(rows["defaultEngine"], "claude");
        assert_eq!(rows["terminalApp"], "iTerm");
        assert_eq!(rows["prPrompt"], "  Keep it short");
        assert_eq!(rows["clickupHiddenStatuses"], r#"["done"]"#);
        assert_eq!(rows["bangs"], "[]");
        assert_eq!(
            Settings::load(&store),
            Settings {
                terminal_app: "iTerm".into(),
                ..settings
            }
        );
    }

    #[test]
    fn rows_from_older_builds_still_read() {
        let rows = |pairs: &[(&str, &str)]| {
            Settings::from_rows(
                pairs
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect(),
            )
        };
        let settings = rows(&[
            ("defaultEngine", "claude"),
            ("runEverything", "true"),
            (
                "scratchMacros",
                r#"[{"id":"m1","name":"Daily standup","prompt":"Go"},{"name":"no id"}]"#,
            ),
        ]);
        assert!(settings.run_everything);
        assert_eq!(settings.default_engine, EngineKind::Claude);
        assert_eq!(
            settings.bangs,
            [Bang {
                id: "m1".into(),
                name: "daily-standup".into(),
                prompt: "Go".into(),
            }]
        );
        // Saved bangs win over the macros they replaced, even an empty list.
        let settings = rows(&[("bangs", "[]"), ("scratchMacros", r#"[{"id":"m1"}]"#)]);
        assert!(settings.bangs.is_empty());
        let settings = rows(&[("defaultEngine", "cursor"), ("notifyApprovals", "maybe")]);
        assert_eq!(settings.default_engine, EngineKind::Claude);
        assert!(settings.notify_approvals);
    }
}
