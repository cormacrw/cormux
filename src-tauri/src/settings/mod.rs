//! The user's app-wide preferences: one typed struct, stored one row per field in the settings
//! table under the field's wire name, so its `settings!` list is the only list of keys, for the
//! struct and its patch alike (and, through its `Default`, of defaults: the webview gets them as
//! `DEFAULT_SETTINGS`).

pub mod commands;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

use rusqlite::{Connection, OptionalExtension};

use crate::engines::EngineKind;
use crate::error::{Error, Result};
use crate::store::{RustMigration, Store};

/// Scratch macros, which bangs replaced.
const LEGACY_MACROS: &str = "scratchMacros";
const BANGS: &str = "bangs";

pub const DEFAULT_WORKTREE_ROOT: &str = "~/.harness/worktrees";

/// Declares `Settings` and `SettingsPatch` from one field list: a patch names any of the fields,
/// and nothing else.
macro_rules! settings {
    ($($(#[doc = $doc:literal])* $field:ident: $ty:ident $(<$item:ident>)?,)*) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
        #[serde(rename_all = "camelCase")]
        pub struct Settings {
            $($(#[doc = $doc])* pub $field: $ty $(<$item>)?,)*
        }

        #[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
        #[serde(rename_all = "camelCase", default, deny_unknown_fields)]
        pub struct SettingsPatch {
            $($(#[doc = $doc])* #[specta(optional)] pub $field: Option<$ty $(<$item>)?>,)*
        }
    };
}

settings! {
    reduce_motion: bool,
    /// Approve every tool request without asking. Questions still wait for the user.
    run_everything: bool,
    /// The experimental References panel, and the tool that pins links to it.
    experimental_references: bool,
    notify_approvals: bool,
    notify_review_finished: bool,
    default_engine: EngineKind,
    /// Preferred repo id for new threads; it may have been removed since.
    default_repo: String,
    /// The folder new worktrees go in, as the user typed it (`~` allowed). Blank saves the
    /// default folder.
    worktree_root: String,
    /// The app Open in Terminal launches; empty means Terminal.
    terminal_app: String,
    /// The app Open in Editor launches; empty means Zed.
    editor_app: String,
    /// Instructions for drafting a PR description; empty means the built-in default.
    pr_prompt: String,
    /// `!name` prompts, for any prompt bar and the command palette.
    bangs: Vec<Bang>,
    clickup_workspace_id: String,
    clickup_space_id: String,
    /// The sprint folder the ClickUp board shows.
    clickup_folder_id: String,
    /// Board lanes the user folded away, by status name.
    clickup_hidden_statuses: Vec<String>,
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
            editor_app: String::new(),
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
    /// A store that can't be read at all gives the defaults, which ask before running anything.
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
        serde_json::from_value(Value::Object(fields)).unwrap_or(defaults)
    }

    /// Saves what `patch` changes, all or nothing. Bang names are tidied into bang shape and
    /// must be there and unique.
    pub fn update(store: &Store, mut patch: SettingsPatch) -> Result<()> {
        for path in [
            &mut patch.worktree_root,
            &mut patch.terminal_app,
            &mut patch.editor_app,
        ]
        .into_iter()
        .flatten()
        {
            *path = path.trim().to_string();
        }
        if let Some(bangs) = &mut patch.bangs {
            check_bangs(bangs)?;
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

    #[cfg(test)]
    pub fn save(&self, store: &Store) {
        let patch = serde_json::from_value(serde_json::to_value(self).unwrap()).unwrap();
        Self::update(store, patch).unwrap();
    }
}

fn stored_text(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

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

/// Older builds saved bangs that today's `Bang` doesn't read: under `scratchMacros` (the scratch
/// macros bangs replaced, named like `Daily standup`) while no bangs were saved, with names not
/// in bang shape, or with items missing their id. They are rewritten once as today's `bangs`.
pub const MIGRATION: RustMigration = RustMigration {
    name: "034_bangs_in_shape",
    run: shape_stored_bangs,
};

fn shape_stored_bangs(conn: &Connection) -> Result<()> {
    let stored = |key: &str| -> Result<Option<String>> {
        Ok(conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    };
    let (saved, macros) = (stored(BANGS)?, stored(LEGACY_MACROS)?);
    if saved.is_none() && macros.is_none() {
        return Ok(());
    }
    let bangs = parse_bangs(saved.as_ref())
        .or_else(|| parse_bangs(macros.as_ref()))
        .unwrap_or_default();
    let text = serde_json::to_string(&bangs).map_err(|error| Error::Store(error.to_string()))?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [BANGS, text.as_str()],
    )?;
    conn.execute("DELETE FROM settings WHERE key = ?1", [LEGACY_MACROS])?;
    Ok(())
}

/// A saved bang list as older builds wrote it; `None` when there's none to read.
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

fn check_bangs(bangs: &mut [Bang]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for bang in bangs {
        bang.name = bang_name(&bang.name);
        if bang.name.is_empty() {
            return Err(Error::Invalid(
                "Give the bang a name, like test or review.".into(),
            ));
        }
        if !seen.insert(bang.name.clone()) {
            return Err(Error::Invalid(format!("!{} already exists.", bang.name)));
        }
    }
    Ok(())
}

/// Bang shape: no `!`, lowercase, spaces as `-`, and only letters, digits, `-` and `_`.
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
    }

    /// Bug: the core saved any bang list, and only one Svelte component kept names unique, so
    /// two drafts renamed alike were both saved.
    #[test]
    fn bang_names_are_tidied_and_must_be_there_and_unique() {
        let store = Store::in_memory().unwrap();
        let bang = |id: &str, name: &str| Bang {
            id: id.into(),
            name: name.into(),
            prompt: "p".into(),
        };
        let save = |bangs: Vec<Bang>| {
            Settings::update(
                &store,
                SettingsPatch {
                    bangs: Some(bangs),
                    ..Default::default()
                },
            )
        };

        save(vec![bang("a", " !Fix  Tests "), bang("b", "lint")]).unwrap();
        let names: Vec<String> = Settings::load(&store)
            .bangs
            .into_iter()
            .map(|bang| bang.name)
            .collect();
        assert_eq!(names, ["fix-tests", "lint"]);

        let taken = save(vec![bang("a", "fix-tests"), bang("b", "Fix tests")]).unwrap_err();
        assert_eq!(taken.to_string(), "!fix-tests already exists.");
        let blank = save(vec![bang("a", " ! ")]).unwrap_err();
        assert!(matches!(blank, Error::Invalid(_)), "{blank}");
        assert_eq!(Settings::load(&store).bangs.len(), 2, "nothing saved");
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
    fn every_field_saves_and_reads_back() {
        let store = Store::in_memory().unwrap();
        let settings = Settings {
            reduce_motion: true,
            run_everything: true,
            experimental_references: true,
            notify_approvals: false,
            notify_review_finished: false,
            default_engine: EngineKind::Claude,
            default_repo: "r1".into(),
            worktree_root: "~/trees".into(),
            terminal_app: "iTerm".into(),
            editor_app: "Cursor".into(),
            pr_prompt: "Keep it short".into(),
            bangs: vec![Bang {
                id: "b1".into(),
                name: "test".into(),
                prompt: "Run the tests".into(),
            }],
            clickup_workspace_id: "team".into(),
            clickup_space_id: "space".into(),
            clickup_folder_id: "folder".into(),
            clickup_hidden_statuses: vec!["done".into()],
        };
        settings.save(&store);
        assert_eq!(Settings::load(&store), settings);
    }

    #[test]
    fn a_patch_naming_no_setting_is_refused() {
        let patch = serde_json::from_value::<SettingsPatch>(
            serde_json::json!({ "reduceMotion": true, "colour": "red" }),
        );
        assert!(patch.is_err());
    }

    #[test]
    fn rows_that_dont_read_leave_the_default() {
        let rows = |pairs: &[(&str, &str)]| {
            Settings::from_rows(
                pairs
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect(),
            )
        };
        let settings = rows(&[("defaultEngine", "claude"), ("runEverything", "true")]);
        assert!(settings.run_everything);
        assert_eq!(settings.default_engine, EngineKind::Claude);
        let settings = rows(&[("defaultEngine", "cursor"), ("notifyApprovals", "maybe")]);
        assert_eq!(settings.default_engine, EngineKind::Claude);
        assert!(settings.notify_approvals);
    }

    #[test]
    fn older_builds_bangs_are_rewritten_once_in_shape() {
        let migrated = |pairs: &[(&str, &str)]| {
            let store = Store::in_memory().unwrap();
            let rows: Vec<(&str, String)> = pairs
                .iter()
                .map(|(key, value)| (*key, value.to_string()))
                .collect();
            store.set_settings(&rows).unwrap();
            store.with_conn(shape_stored_bangs).unwrap();
            let keys: Vec<String> = store
                .with_read(crate::store::settings_in)
                .unwrap()
                .into_iter()
                .map(|(key, _)| key)
                .collect();
            assert!(!keys.contains(&LEGACY_MACROS.to_string()), "{keys:?}");
            Settings::load(&store).bangs
        };
        assert_eq!(
            migrated(&[(
                "scratchMacros",
                r#"[{"id":"m1","name":"Daily standup","prompt":"Go"},{"name":"no id"}]"#,
            )]),
            [Bang {
                id: "m1".into(),
                name: "daily-standup".into(),
                prompt: "Go".into(),
            }]
        );
        assert!(migrated(&[("bangs", "[]"), ("scratchMacros", r#"[{"id":"m1"}]"#)]).is_empty());
        assert_eq!(
            migrated(&[("bangs", r#"[{"id":"b1","name":"!Fix It"}]"#)]),
            [Bang {
                id: "b1".into(),
                name: "fix-it".into(),
                prompt: String::new(),
            }]
        );
        assert!(migrated(&[]).is_empty());
    }
}
