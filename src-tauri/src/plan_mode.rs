//! Plan mode: a thread's agent reads and plans, but edits nothing until the user approves
//! the plan it hands over. Only Claude has it, as a permission mode.

use std::path::Path;

use serde_json::{Value, json};

use crate::error::Result;
use crate::state::AppState;
use crate::store::Store;
use crate::workspace::ThreadActivity;

/// The tool Claude calls to hand a finished plan over for approval.
pub const EXIT_PLAN_TOOL: &str = "ExitPlanMode";

fn plan_mode_key(thread_id: &str) -> String {
    format!("thread:{thread_id}/planMode")
}

/// Whether the user switched the thread to plan mode. The composer reads the same setting.
pub fn is_plan_mode(store: &Store, thread_id: &str) -> bool {
    store
        .get_setting(&plan_mode_key(thread_id))
        .ok()
        .flatten()
        .as_deref()
        == Some("true")
}

pub fn set_plan_mode(store: &Store, thread_id: &str, on: bool) -> Result<()> {
    store.set_setting(&plan_mode_key(thread_id), if on { "true" } else { "false" })
}

/// Claude's permission mode for plan mode.
pub const PLAN_MODE: &str = "plan";

fn normal_mode_key(thread_id: &str) -> String {
    format!("thread:{thread_id}/normalMode")
}

/// Notes the mode a session started in outside plan mode, such as `auto`, so leaving plan
/// mode can go back to it.
pub fn remember_normal_mode(store: &Store, thread_id: &str, mode: &str) {
    if mode != PLAN_MODE {
        let _ = store.set_setting(&normal_mode_key(thread_id), mode);
    }
}

/// The mode the thread's agent works in outside plan mode: what its last session started in,
/// else the default Claude's settings give the worktree, else `default`.
pub fn normal_mode(store: &Store, thread_id: &str, cwd: &Path) -> String {
    store
        .get_setting(&normal_mode_key(thread_id))
        .ok()
        .flatten()
        .or_else(|| {
            let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
            configured_default_mode(cwd, home.as_deref())
        })
        .unwrap_or_else(|| "default".into())
}

/// `permissions.defaultMode` from Claude's settings, in its order: the worktree's local
/// settings, then its shared ones, then the user's.
fn configured_default_mode(cwd: &Path, home: Option<&Path>) -> Option<String> {
    let mut files = vec![
        cwd.join(".claude/settings.local.json"),
        cwd.join(".claude/settings.json"),
    ];
    if let Some(home) = home {
        files.push(home.join(".claude/settings.json"));
    }
    files.iter().find_map(|path| {
        let text = std::fs::read_to_string(path).ok()?;
        let value: Value = serde_json::from_str(&text).ok()?;
        let mode = value.pointer("/permissions/defaultMode")?.as_str()?;
        (mode != PLAN_MODE).then(|| mode.to_string())
    })
}

/// The markdown plan from an `ExitPlanMode` call.
pub fn plan_from_input(input: &Value) -> Option<String> {
    let plan = input.get("plan")?.as_str()?.trim();
    (!plan.is_empty()).then(|| plan.to_string())
}

/// A one-line name for a plan: its first heading, else its first line.
pub fn plan_title(plan: &str) -> String {
    let lines = || plan.lines().map(str::trim).filter(|line| !line.is_empty());
    let line = lines()
        .find(|line| line.starts_with('#'))
        .or_else(|| lines().next())
        .unwrap_or_default();
    let title = line.trim_start_matches('#').trim();
    let title = title.strip_prefix("Plan:").unwrap_or(title).trim();
    if title.is_empty() {
        "Plan".into()
    } else {
        title.to_string()
    }
}

/// Sent with the allow that approves a plan, so Claude carries it out in the thread's normal
/// mode (see `normal_mode`).
pub fn leave_plan_mode_permissions(normal_mode: &str) -> Value {
    json!([{ "type": "setMode", "mode": normal_mode, "destination": "session" }])
}

/// The approval key a plan is recorded under (see `approvals::payload`).
const PLAN_APPROVAL_TOOL: &str = "execute_plan";

/// What Claude hears when the user leaves plan mode while its plan waits.
const LEFT_PLAN_MODE: &str = "I switched you out of plan mode, so I won't be reviewing this plan. Don't wait for approval; carry on in normal mode.";

/// Leaving plan mode drops the thread's waiting plan: Claude is told, and goes back to work.
/// Returns whether there was one.
pub async fn dismiss_waiting_plan(state: &AppState, thread_id: &str) -> Result<bool> {
    let waiting: Vec<_> = state
        .store
        .pending_approvals_for_thread(thread_id)?
        .into_iter()
        .filter(|row| row.tool == PLAN_APPROVAL_TOOL)
        .collect();
    for row in &waiting {
        crate::approvals::mark_dismissed(&state.store, &row.id)?;
        // No engine waiting (the app restarted since) leaves nothing to tell.
        let _ = state
            .approvals
            .resolve(&row.id, false, Some(LEFT_PLAN_MODE.into()))
            .await;
    }
    if !waiting.is_empty() {
        resume_after_review(state, thread_id).await?;
    }
    Ok(!waiting.is_empty())
}

/// A thread whose plan is waiting for the user isn't working: its turn is on hold until the
/// user answers. Moves such threads from running to waiting, which the workspace shows as
/// needing attention.
pub async fn hold_threads_awaiting_review(state: &AppState) -> Result<()> {
    let snapshot = state.store.snapshot()?;
    for approval in snapshot
        .approvals
        .iter()
        .filter(|row| row.status == "pending" && row.tool == PLAN_APPROVAL_TOOL)
    {
        let Some(thread) = snapshot
            .threads
            .iter()
            .find(|thread| thread.id == approval.thread_id && thread.status == "running")
        else {
            continue;
        };
        set_activity(state, &thread.id, &thread.workspace_id, "waiting").await?;
    }
    Ok(())
}

/// Once the user answers a plan, the agent works again: revising it, or carrying it out.
pub async fn resume_after_review(state: &AppState, thread_id: &str) -> Result<()> {
    let Some(thread) = state.store.thread_by_id(thread_id)? else {
        return Ok(());
    };
    if thread.status == "waiting" {
        set_activity(state, &thread.id, &thread.workspace_id, "running").await?;
    }
    Ok(())
}

async fn set_activity(
    state: &AppState,
    thread_id: &str,
    workspace_id: &str,
    status: &str,
) -> Result<()> {
    state.store.set_thread_status(thread_id, status)?;
    let activity = if status == "waiting" {
        ThreadActivity::Waiting
    } else {
        ThreadActivity::Running
    };
    state
        .workspace
        .set_thread(thread_id, workspace_id, activity)
        .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_plan_from_exit_plan_mode_input() {
        let input = json!({"plan": "\n# Plan: add retries\n\n1. Wrap the call\n", "planFilePath": "/tmp/p.md"});
        assert_eq!(
            plan_from_input(&input).as_deref(),
            Some("# Plan: add retries\n\n1. Wrap the call")
        );
        assert_eq!(plan_from_input(&json!({"plan": "  "})), None);
        assert_eq!(plan_from_input(&json!({})), None);
    }

    #[test]
    fn titles_a_plan_by_its_first_heading() {
        assert_eq!(plan_title("# Plan: add retries\n\nBody"), "add retries");
        assert_eq!(plan_title("Intro line\n## Steps\n1. a"), "Steps");
        assert_eq!(plan_title("Just do it\nthen this"), "Just do it");
        assert_eq!(plan_title("#\n"), "Plan");
    }

    #[test]
    fn normal_mode_comes_from_the_last_session_then_claudes_settings() {
        let root = std::env::temp_dir().join(format!("cormux-modes-{}", std::process::id()));
        let (cwd, home) = (root.join("worktree"), root.join("home"));
        let write = |path: std::path::PathBuf, mode: &str| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                format!(r#"{{"permissions":{{"defaultMode":"{mode}"}}}}"#),
            )
            .unwrap();
        };
        assert_eq!(configured_default_mode(&cwd, Some(&home)), None);
        write(home.join(".claude/settings.json"), "auto");
        assert_eq!(
            configured_default_mode(&cwd, Some(&home)).as_deref(),
            Some("auto")
        );
        write(cwd.join(".claude/settings.json"), "acceptEdits");
        assert_eq!(
            configured_default_mode(&cwd, Some(&home)).as_deref(),
            Some("acceptEdits")
        );
        // A default of plan mode is no normal mode to go back to.
        write(cwd.join(".claude/settings.local.json"), "plan");
        assert_eq!(
            configured_default_mode(&cwd, Some(&home)).as_deref(),
            Some("acceptEdits")
        );

        let store = Store::new();
        store.open_in_memory().unwrap();
        remember_normal_mode(&store, "t1", "plan");
        remember_normal_mode(&store, "t1", "bypassPermissions");
        assert_eq!(normal_mode(&store, "t1", &cwd), "bypassPermissions");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_mode_is_stored_per_thread() {
        let store = Store::new();
        store.open_in_memory().unwrap();
        assert!(!is_plan_mode(&store, "t1"));
        set_plan_mode(&store, "t1", true).unwrap();
        assert!(is_plan_mode(&store, "t1"));
        assert!(!is_plan_mode(&store, "t2"));
        set_plan_mode(&store, "t1", false).unwrap();
        assert!(!is_plan_mode(&store, "t1"));
    }
}
