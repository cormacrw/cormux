//! The user's app-wide preferences: one typed struct, stored one row per field in the settings
//! table under the keys below, which nothing else names.

pub mod commands;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::engines::EngineKind;
use crate::error::Result;
use crate::store::Store;

const REDUCE_MOTION: &str = "reduceMotion";
const RUN_EVERYTHING: &str = "runEverything";
const EXPERIMENTAL_REFERENCES: &str = "experimentalReferences";
const NOTIFY_APPROVALS: &str = "notifyApprovals";
const NOTIFY_REVIEW_FINISHED: &str = "notifyReviewFinished";
const DEFAULT_ENGINE: &str = "defaultEngine";
const DEFAULT_REPO: &str = "defaultRepo";
const WORKTREE_ROOT: &str = "worktreeRoot";
const TERMINAL_APP: &str = "terminalApp";
const PR_PROMPT: &str = "prPrompt";
const BANGS: &str = "bangs";
/// Scratch macros, which bangs replaced; read while no bangs are saved.
const LEGACY_MACROS: &str = "scratchMacros";
const CLICKUP_WORKSPACE: &str = "clickupWorkspaceId";
const CLICKUP_SPACE: &str = "clickupSpaceId";
const CLICKUP_FOLDER: &str = "clickupFolderId";
const CLICKUP_HIDDEN_STATUSES: &str = "clickupHiddenStatuses";

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

    fn from_rows(rows: HashMap<String, String>) -> Self {
        let defaults = Self::default();
        let text = |key: &str| rows.get(key).map(|value| value.trim()).unwrap_or("");
        let flag = |key: &str, default: bool| match text(key) {
            "true" => true,
            "false" => false,
            _ => default,
        };
        let string = |key: &str, default: String| match text(key) {
            "" => default,
            value => value.to_string(),
        };
        let bangs = parse_bangs(rows.get(BANGS))
            .or_else(|| parse_bangs(rows.get(LEGACY_MACROS)))
            .unwrap_or_default();
        Self {
            reduce_motion: flag(REDUCE_MOTION, defaults.reduce_motion),
            run_everything: flag(RUN_EVERYTHING, defaults.run_everything),
            experimental_references: flag(
                EXPERIMENTAL_REFERENCES,
                defaults.experimental_references,
            ),
            notify_approvals: flag(NOTIFY_APPROVALS, defaults.notify_approvals),
            notify_review_finished: flag(NOTIFY_REVIEW_FINISHED, defaults.notify_review_finished),
            default_engine: serde_json::from_value(text(DEFAULT_ENGINE).into())
                .unwrap_or(defaults.default_engine),
            default_repo: string(DEFAULT_REPO, defaults.default_repo),
            worktree_root: string(WORKTREE_ROOT, defaults.worktree_root),
            terminal_app: string(TERMINAL_APP, defaults.terminal_app),
            // Not trimmed: the user's own text.
            pr_prompt: rows.get(PR_PROMPT).cloned().unwrap_or_default(),
            bangs,
            clickup_workspace_id: string(CLICKUP_WORKSPACE, defaults.clickup_workspace_id),
            clickup_space_id: string(CLICKUP_SPACE, defaults.clickup_space_id),
            clickup_folder_id: string(CLICKUP_FOLDER, defaults.clickup_folder_id),
            clickup_hidden_statuses: rows
                .get(CLICKUP_HIDDEN_STATUSES)
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_default(),
        }
    }

    /// Saves what `patch` changes, all or nothing.
    pub fn update(store: &Store, patch: SettingsPatch) -> Result<()> {
        let flag = |on: bool| if on { "true" } else { "false" }.to_string();
        let mut rows: Vec<(&str, String)> = Vec::new();
        let mut put = |key: &'static str, value: Option<String>| {
            if let Some(value) = value {
                rows.push((key, value));
            }
        };
        put(REDUCE_MOTION, patch.reduce_motion.map(flag));
        put(RUN_EVERYTHING, patch.run_everything.map(flag));
        put(
            EXPERIMENTAL_REFERENCES,
            patch.experimental_references.map(flag),
        );
        put(NOTIFY_APPROVALS, patch.notify_approvals.map(flag));
        put(
            NOTIFY_REVIEW_FINISHED,
            patch.notify_review_finished.map(flag),
        );
        put(
            DEFAULT_ENGINE,
            patch.default_engine.map(|engine| word(&engine)),
        );
        put(DEFAULT_REPO, patch.default_repo);
        put(
            WORKTREE_ROOT,
            patch.worktree_root.map(|root| root.trim().into()),
        );
        put(
            TERMINAL_APP,
            patch.terminal_app.map(|app| app.trim().into()),
        );
        put(PR_PROMPT, patch.pr_prompt);
        put(BANGS, patch.bangs.map(|bangs| json(&bangs)));
        put(CLICKUP_WORKSPACE, patch.clickup_workspace_id);
        put(CLICKUP_SPACE, patch.clickup_space_id);
        put(CLICKUP_FOLDER, patch.clickup_folder_id);
        put(
            CLICKUP_HIDDEN_STATUSES,
            patch.clickup_hidden_statuses.map(|names| json(&names)),
        );
        store.set_settings(&rows)
    }

    /// The default engine as threads store it.
    pub fn default_engine_name(&self) -> String {
        word(&self.default_engine)
    }

    #[cfg(test)]
    pub fn save(&self, store: &Store) {
        Self::update(
            store,
            SettingsPatch {
                run_everything: Some(self.run_everything),
                experimental_references: Some(self.experimental_references),
                ..Default::default()
            },
        )
        .unwrap();
    }
}

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("plain data serializes")
}

/// An enum's wire word, stored without JSON quotes.
fn word(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(word)) => word,
        _ => json(value),
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
