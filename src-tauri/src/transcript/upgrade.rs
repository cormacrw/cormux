//! Rewrites rows older builds saved into today's events, once, so nothing reads the old shapes:
//! core steps saved as tool calls titled in English (a `control-`, `legacy-`, `pr-open-`,
//! `scratch-open-`, `review-` or `finding-` id, or a bare `{icon, title, detail}`) become typed
//! steps; tool calls and permission requests get the role their Claude tool name meant; replayed
//! tool calls are dropped; a review's untyped briefing gets its type (024); a permission request
//! saved with a title gets its card (037). The rules are the ones the timeline drew these rows
//! by, so they look the same afterwards.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use serde_json::{Map, Value};

use super::{HostEvent, StepEvent, TranscriptEvent};
use crate::engines::{
    AgentEvent, SavedCall, ToolCallStatus, restore_call, restore_permission_tool,
};
use crate::error::Result;
use crate::model::ApprovalPayload;
use crate::store::RustMigration;

use super::store::{Rewrite, rewrite_rows};
use crate::timeline::{StepIcon, TimelineStep};

pub const MIGRATION: RustMigration = RustMigration {
    name: "024_typed_transcript_rows",
    run,
};

const NEW_SESSION_TITLE: &str = "Started a new session";
const RESUME_TITLE: &str = "Resumed an earlier conversation";
/// Ids the core gave its own steps before they were typed. `finding-` ones drew as tool steps,
/// the rest as dividers.
const DIVIDER_IDS: [&str; 5] = [
    "control-",
    "legacy-",
    "pr-open-",
    "scratch-open-",
    "review-",
];
const FINDING_ID: &str = "finding-";

fn run(conn: &Connection) -> Result<()> {
    let mut seen: HashMap<String, HashSet<String>> = HashMap::new();
    rewrite_rows(
        conn,
        |thread_id, seq, kind, payload| {
            let ids = seen.entry(thread_id.to_string()).or_default();
            rewrite_event(ids, seq, kind, payload)
        },
        rewrite_approval,
    )
}

fn rewrite_event(ids: &mut HashSet<String>, seq: i64, kind: &str, payload: &str) -> Rewrite {
    let Ok(Value::Object(row)) = serde_json::from_str::<Value>(payload) else {
        return Rewrite::Keep;
    };
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    let rewritten = match text("type") {
        None if text("role") == Some("user") => {
            TranscriptEvent::briefing(text("text").unwrap_or_default())
        }
        None => match text("title") {
            Some(title) => divider_step(&format!("legacy-{seq}"), title, text("detail")),
            None => return Rewrite::Keep,
        },
        Some("toolCall") => {
            let id = text("id").unwrap_or_default().to_string();
            if !ids.insert(id.clone()) {
                return Rewrite::Delete;
            }
            tool_call(&id, &row)
        }
        Some("step") => {
            if !ids.insert(text("id").unwrap_or_default().to_string()) {
                return Rewrite::Delete;
            }
            return Rewrite::Keep;
        }
        Some("permission") if row.contains_key("tool_name") => permission(&row),
        _ => return Rewrite::Keep,
    };
    match serde_json::to_string(&rewritten) {
        Ok(payload) => Rewrite::Replace {
            kind: kind.to_string(),
            payload,
        },
        Err(_) => Rewrite::Keep,
    }
}

fn tool_call(id: &str, row: &Map<String, Value>) -> TranscriptEvent {
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    let title = text("title").unwrap_or_default();
    let detail = text("detail");
    if DIVIDER_IDS.iter().any(|prefix| id.starts_with(prefix)) {
        return divider_step(id, title, detail);
    }
    if id.starts_with(FINDING_ID) {
        return finding_step(id, title, detail);
    }
    let restored = restore_call(&SavedCall {
        name: text("name"),
        title,
        kind: text("kind").unwrap_or("other"),
        detail,
    });
    let status = row
        .get("status")
        .cloned()
        .and_then(|status| serde_json::from_value(status).ok())
        .unwrap_or(ToolCallStatus::Completed);
    let locations = row
        .get("locations")
        .cloned()
        .and_then(|locations| serde_json::from_value(locations).ok())
        .unwrap_or_default();
    AgentEvent::ToolCall {
        id: id.to_string(),
        title: restored.title,
        role: restored.role,
        status,
        locations,
        detail: detail.map(str::to_string),
        hidden: restored.hidden,
        parent_id: text("parent_id").map(str::to_string),
    }
    .into()
}

fn permission(row: &Map<String, Value>) -> TranscriptEvent {
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    let detail = text("detail");
    let (tool, role) = restore_permission_tool(
        text("tool_name").unwrap_or_default(),
        text("kind").unwrap_or("other"),
        detail,
    );
    HostEvent::Permission {
        id: text("id").unwrap_or_default().to_string(),
        tool_call_id: text("tool_call_id").map(str::to_string),
        card: Box::new(ApprovalPayload::to_use(&tool, detail)),
        tool,
        role,
        detail: detail.map(str::to_string),
        auto_approved: row
            .get("auto_approved")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
    .into()
}

/// An approval card named the raw tool, which the timeline swapped for its words. A question's
/// card names no tool.
fn rewrite_approval(payload: &str) -> Option<String> {
    let mut card: Map<String, Value> = serde_json::from_str(payload).ok()?;
    if card.contains_key("questions") {
        return None;
    }
    let raw = card.get("why").and_then(Value::as_str)?.to_string();
    let (words, _) = restore_permission_tool(&raw, "other", None);
    if raw.is_empty() || words == raw {
        return None;
    }
    for key in ["title", "what", "why"] {
        if let Some(Value::String(text)) = card.get_mut(key) {
            *text = text.replacen(&raw, &words, 1);
        }
    }
    serde_json::to_string(&card).ok()
}

/// A step the timeline drew between turns, by what its title said.
fn divider_step(id: &str, title: &str, detail: Option<&str>) -> TranscriptEvent {
    let session = id
        .starts_with("control-")
        .then(|| session_step(title, detail));
    let step = match session.flatten() {
        Some(step) => step,
        None => match divider_icon(title) {
            StepIcon::Pause => TimelineStep::AgentPaused,
            StepIcon::Play => TimelineStep::AgentResumed,
            StepIcon::Stop => TimelineStep::AgentStopped,
            icon => TimelineStep::Legacy {
                title: title.to_string(),
                icon,
                run: false,
            },
        },
    };
    step_event(id, title, detail, step)
}

fn session_step(title: &str, detail: Option<&str>) -> Option<TimelineStep> {
    let detail: Map<String, Value> = detail
        .and_then(|detail| serde_json::from_str(detail).ok())
        .unwrap_or_default();
    let field = |key: &str| detail.get(key).and_then(Value::as_str).map(str::to_string);
    match title {
        NEW_SESSION_TITLE => Some(TimelineStep::SessionStarted {
            ended_session: field("endedSession"),
        }),
        RESUME_TITLE => Some(TimelineStep::SessionResumed {
            conversation: field("conversation").unwrap_or_default(),
            ended_session: field("endedSession"),
        }),
        _ => None,
    }
}

/// A step the timeline drew among the agent's tool steps.
fn finding_step(id: &str, title: &str, detail: Option<&str>) -> TranscriptEvent {
    let lower = title.to_lowercase();
    let step = if let Some(count) = number_after(&lower, "received ", " findings from the review") {
        TimelineStep::FindingsReceived {
            count,
            titles: Vec::new(),
        }
    } else if let Some(count) = lower
        .strip_prefix("fixed ")
        .and_then(|rest| number_after(rest, "", " findings"))
    {
        TimelineStep::FindingsFixed {
            count,
            files: Vec::new(),
        }
    } else {
        TimelineStep::Legacy {
            title: title.to_string(),
            icon: tool_icon(&lower),
            run: true,
        }
    };
    step_event(id, title, detail, step)
}

fn step_event(id: &str, title: &str, detail: Option<&str>, step: TimelineStep) -> TranscriptEvent {
    TranscriptEvent::Step(StepEvent::Step {
        id: id.to_string(),
        title: title.to_string(),
        detail: detail.map(str::to_string),
        step,
    })
}

fn divider_icon(title: &str) -> StepIcon {
    let lower = title.to_lowercase();
    let starts = |prefix: &str| lower.starts_with(prefix);
    if lower.contains("you paused") {
        StepIcon::Pause
    } else if lower.contains("you resumed") {
        StepIcon::Play
    } else if lower.contains("you stopped") {
        StepIcon::Stop
    } else if lower.contains("new session") {
        StepIcon::Session
    } else if starts("pulled ") {
        StepIcon::Download
    } else if starts("switched") || starts("rebased") || lower.contains("worktree") {
        StepIcon::Branch
    } else if has_issue_number(&lower) || lower.contains("suggestions") {
        StepIcon::Pr
    } else if lower
        .strip_prefix("read ")
        .is_some_and(|rest| number_after(rest, "", " changed").is_some())
    {
        StepIcon::File
    } else if starts("deleted ") {
        StepIcon::Trash
    } else if lower.contains("migration") {
        StepIcon::Database
    } else if lower.contains("approved") {
        StepIcon::Check
    } else if starts("opened ") {
        StepIcon::File
    } else {
        StepIcon::Tool
    }
}

/// How a `finding-` step that wasn't a findings count drew, from its lower-cased title.
fn tool_icon(lower: &str) -> StepIcon {
    if lower.contains("you paused") {
        StepIcon::Pause
    } else if lower.contains("you resumed") {
        StepIcon::Play
    } else if lower.contains("branch") {
        StepIcon::Branch
    } else if lower.contains("pull") || lower.contains("pr") {
        StepIcon::Pr
    } else if lower.contains("database") || lower.contains("migration") {
        StepIcon::Database
    } else {
        StepIcon::Tool
    }
}

/// `#` and a digit anywhere, as in `PR #12`.
fn has_issue_number(text: &str) -> bool {
    text.as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'#' && pair[1].is_ascii_digit())
}

/// The number in `{before}{digits}{after}` somewhere in `text` (at its start when `before` is
/// empty).
fn number_after(text: &str, before: &str, after: &str) -> Option<u32> {
    let mut search = text;
    loop {
        let at = if before.is_empty() {
            0
        } else {
            search.find(before)? + before.len()
        };
        let rest = &search[at..];
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits > 0 && rest[digits..].starts_with(after) {
            return rest[..digits].parse().ok();
        }
        if before.is_empty() {
            return None;
        }
        search = &search[at..];
    }
}

/// Permission requests saved with a title instead of their card get the card the timeline drew
/// for them from the request's tool and detail.
pub const PERMISSION_CARDS: RustMigration = RustMigration {
    name: "037_permission_cards",
    run: |conn| {
        rewrite_rows(
            conn,
            |_, _, kind, payload| with_card(kind, payload),
            |_| None,
        )
    },
};

fn with_card(kind: &str, payload: &str) -> Rewrite {
    let Ok(Value::Object(mut row)) = serde_json::from_str::<Value>(payload) else {
        return Rewrite::Keep;
    };
    if row.get("type").and_then(Value::as_str) != Some("permission") || row.contains_key("card") {
        return Rewrite::Keep;
    }
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    let card = ApprovalPayload::to_use(text("tool").unwrap_or_default(), text("detail"));
    let Ok(card) = serde_json::to_value(card) else {
        return Rewrite::Keep;
    };
    row.remove("title");
    row.insert("card".into(), card);
    Rewrite::Replace {
        kind: kind.to_string(),
        payload: Value::Object(row).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::engines::ToolRole;
    use crate::model::ThreadRole;
    use crate::store::Store;
    use crate::store::types::ThreadRow;

    fn store_with(rows: &[(&str, Value)]) -> Store {
        let store = Store::in_memory().unwrap();
        store
            .upsert_thread(&ThreadRow::new("t1", "w1", ThreadRole::Lead, "claude"))
            .unwrap();
        for (kind, payload) in rows {
            store
                .append_event("t1", kind, &payload.to_string())
                .unwrap();
        }
        store
    }

    /// The typed rows; anything still untyped is left out.
    fn events(store: &Store) -> Vec<(i64, TranscriptEvent)> {
        store
            .thread_events_after("t1", 0, 100)
            .unwrap()
            .into_iter()
            .filter_map(|(seq, _, payload)| Some((seq, serde_json::from_str(&payload).ok()?)))
            .collect()
    }

    fn step_of(event: &TranscriptEvent) -> (&str, &str, Option<&str>, &TimelineStep) {
        match event {
            TranscriptEvent::Step(StepEvent::Step {
                id,
                title,
                detail,
                step,
            }) => (id, title, detail.as_deref(), step),
            other => panic!("not a step: {other:?}"),
        }
    }

    #[test]
    fn core_steps_saved_as_titled_tool_calls_become_typed_steps() {
        let call = |id: &str, title: &str, detail: Value| {
            (
                "tool",
                json!({ "type": "toolCall", "id": id, "title": title, "name": null,
                        "kind": "other", "status": "completed", "locations": [], "detail": detail }),
            )
        };
        let store = store_with(&[
            call("control-1", "You paused the agent", Value::Null),
            call(
                "control-2",
                "Started a new session",
                json!(r#"{"endedSession":"sess-1"}"#),
            ),
            call(
                "control-3",
                "Resumed an earlier conversation",
                json!(r#"{"conversation":"after-2","endedSession":"sess-2"}"#),
            ),
            call("pr-open-12", "Opened PR #12", json!("feat → main")),
            call("legacy-x", "Pulled 3 commits from main", Value::Null),
            call("review-a1", "Approved the plan", Value::Null),
            call(
                "finding-1",
                "Received 2 findings from the review",
                json!("A · B"),
            ),
            call("finding-2", "Fixed 2 findings", json!("a.rs, b.rs")),
            call("finding-3", "Sent the findings to the branch", Value::Null),
            (
                "tool",
                json!({ "icon": "branch", "title": "Switched to `b`", "detail": "x" }),
            ),
            ("tool", json!({ "icon": "x" })),
        ]);
        store.with_conn(run).unwrap();
        let rows = events(&store);

        let (_, title, _, step) = step_of(&rows[0].1);
        assert_eq!(
            (title, step),
            ("You paused the agent", &TimelineStep::AgentPaused)
        );
        assert_eq!(
            step_of(&rows[1].1).3,
            &TimelineStep::SessionStarted {
                ended_session: Some("sess-1".into()),
            }
        );
        assert_eq!(
            step_of(&rows[2].1).3,
            &TimelineStep::SessionResumed {
                conversation: "after-2".into(),
                ended_session: Some("sess-2".into()),
            }
        );
        let earlier = |icon, run| TimelineStep::Legacy {
            title: String::new(),
            icon,
            run,
        };
        let look = |step: &TimelineStep| match step {
            TimelineStep::Legacy { icon, run, .. } => earlier(*icon, *run),
            other => other.clone(),
        };
        let (id, title, detail, step) = step_of(&rows[3].1);
        assert_eq!(
            (id, title, detail),
            ("pr-open-12", "Opened PR #12", Some("feat → main"))
        );
        assert_eq!(look(step), earlier(StepIcon::Pr, false));
        assert_eq!(
            look(step_of(&rows[4].1).3),
            earlier(StepIcon::Download, false)
        );
        assert_eq!(look(step_of(&rows[5].1).3), earlier(StepIcon::Check, false));
        assert_eq!(
            step_of(&rows[6].1).3,
            &TimelineStep::FindingsReceived {
                count: 2,
                titles: Vec::new(),
            }
        );
        assert_eq!(
            step_of(&rows[7].1).3,
            &TimelineStep::FindingsFixed {
                count: 2,
                files: Vec::new(),
            }
        );
        assert_eq!(look(step_of(&rows[8].1).3), earlier(StepIcon::Branch, true));
        let (id, title, detail, step) = step_of(&rows[9].1);
        assert_eq!(
            (id, title, detail),
            ("legacy-10", "Switched to `b`", Some("x"))
        );
        assert_eq!(look(step), earlier(StepIcon::Branch, false));
        assert_eq!(rows.len(), 10, "a row with nothing to show is left alone");
        assert_eq!(store.thread_events_after("t1", 0, 100).unwrap().len(), 11);
    }

    #[test]
    fn saved_tool_calls_get_their_roles_and_replays_go() {
        let store = store_with(&[
            (
                "tool",
                json!({ "type": "toolCall", "id": "toolu_1", "title": "npm run preview",
                        "name": "Bash", "kind": "execute", "status": "completed",
                        "locations": [], "detail": "Preview" }),
            ),
            (
                "tool",
                json!({ "type": "toolCall", "id": "toolu_1", "title": "npm run preview",
                        "name": "Bash", "kind": "execute", "status": "completed",
                        "locations": [], "detail": "Preview" }),
            ),
            (
                "tool",
                json!({ "type": "toolCall", "id": "toolu_2", "title": "Task", "name": "Task",
                        "kind": "other", "status": "inProgress", "locations": [],
                        "detail": "Find auth" }),
            ),
            (
                "tool",
                json!({ "type": "toolCall", "id": "toolu_3", "title": "ExitPlanMode",
                        "name": "ExitPlanMode", "kind": "other", "status": "pending",
                        "locations": [], "detail": null }),
            ),
            (
                "permission",
                json!({ "type": "permission", "id": "p1", "tool_call_id": "toolu_4",
                        "title": "Use mcp__github__get_pr", "tool_name": "mcp__github__get_pr",
                        "kind": "other", "detail": null, "auto_approved": false }),
            ),
            ("message", json!({ "role": "user", "text": "Review PR #4" })),
        ]);
        store
            .insert_pending_approval("p1", "t1", "other", &ApprovalPayload::titled("Legacy"))
            .unwrap();
        let legacy = json!({ "title": "Use mcp__github__get_pr",
                             "what": "mcp__github__get_pr", "why": "mcp__github__get_pr",
                             "okLabel": "Approve", "noLabel": "Deny" });
        store
            .set_approval_payload_text("p1", &legacy.to_string())
            .unwrap();
        store.with_conn(run).unwrap();
        let rows = events(&store);

        assert_eq!(rows.len(), 5, "the replayed call is gone");
        let TranscriptEvent::Agent(AgentEvent::ToolCall { role, .. }) = &rows[0].1 else {
            panic!("{:?}", rows[0]);
        };
        assert!(matches!(role, ToolRole::Command { command, .. } if command == "npm run preview"));
        let TranscriptEvent::Agent(AgentEvent::ToolCall { role, .. }) = &rows[1].1 else {
            panic!("{:?}", rows[1]);
        };
        assert!(matches!(role, ToolRole::Subagent { .. }));
        let TranscriptEvent::Agent(AgentEvent::ToolCall { hidden, .. }) = &rows[2].1 else {
            panic!("{:?}", rows[2]);
        };
        assert!(hidden);
        let TranscriptEvent::Host(HostEvent::Permission { card, tool, .. }) = &rows[3].1 else {
            panic!("{:?}", rows[3]);
        };
        assert_eq!(
            (card.title.as_str(), tool.as_str()),
            ("Use Github: Get pr", "Github: Get pr")
        );
        assert_eq!(rows[4].1, TranscriptEvent::briefing("Review PR #4"));

        let card = store.get_approval("p1").unwrap().unwrap().payload.unwrap();
        assert_eq!(card.title, "Use Github: Get pr");
        assert_eq!(card.why, "Github: Get pr");
    }

    #[test]
    fn a_permission_saved_with_a_title_gets_the_card_it_was_drawn_with() {
        let store = store_with(&[(
            "permission",
            json!({ "type": "permission", "id": "p1", "tool_call_id": null,
                    "title": "Use Bash", "tool": "Bash",
                    "role": { "kind": "command", "command": "ls", "description": null },
                    "detail": "ls", "auto_approved": false }),
        )]);
        store.with_conn(PERMISSION_CARDS.run).unwrap();
        store.with_conn(PERMISSION_CARDS.run).unwrap();

        let rows = events(&store);
        let TranscriptEvent::Host(HostEvent::Permission { card, .. }) = &rows[0].1 else {
            panic!("{:?}", rows[0]);
        };
        assert_eq!(**card, ApprovalPayload::to_use("Bash", Some("ls")));
        assert_eq!(
            (card.title.as_str(), card.what.as_str(), card.why.as_str()),
            ("Use Bash", "ls", "Bash")
        );
        let (_, _, saved) = store.thread_events_after("t1", 0, 1).unwrap().remove(0);
        let saved: Value = serde_json::from_str(&saved).unwrap();
        assert!(saved.get("title").is_none(), "{saved}");
    }

    #[test]
    fn opening_an_older_database_rewrites_its_rows_once_in_version_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cormux.db");
        {
            let conn = Connection::open(&path).unwrap();
            crate::store::migrate::migrated_before(&conn, 24);
            conn.execute_batch(
                r#"INSERT INTO threads (id, workspace_id, title, engine, status)
                    VALUES ('t1', 'w1', 'Lead', 'claude', 'idle');
                 INSERT INTO thread_events (thread_id, seq, kind, payload)
                    VALUES ('t1', 1, 'tool', '{"icon":"pause","title":"You paused the agent"}');"#,
            )
            .unwrap();
        }

        let store = Store::open(&path, &[MIGRATION]).unwrap();
        let rows = events(&store);
        assert_eq!(step_of(&rows[0].1).3, &TimelineStep::AgentPaused);
        let ledger: Vec<i64> = store
            .with_read(|conn| {
                Ok(conn
                    .prepare(
                        "SELECT version FROM schema_migrations WHERE version >= 23 ORDER BY rowid",
                    )?
                    .query_map([], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<i64>>>()?)
            })
            .unwrap();
        assert_eq!(ledger[..3], [23, 24, 25]);

        store
            .append_event("t1", "tool", &json!({ "title": "Later" }).to_string())
            .unwrap();
        drop(store);
        let store = Store::open(&path, &[MIGRATION]).unwrap();
        assert!(
            store.thread_events_after("t1", 1, 10).unwrap()[0]
                .2
                .contains("Later"),
            "not rewritten a second time"
        );
    }
}
