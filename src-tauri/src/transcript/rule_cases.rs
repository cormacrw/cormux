//! The transcript rules the browser harness re-implements (`src/lib/dev/harness/db.ts`), checked
//! against the cases both test suites read: `src/lib/ipc/rule-cases/transcript.json`.

use serde::Deserialize;
use serde_json::{Value, json};

use super::Transcript;
use crate::model::{ThreadStatus, WorkspaceLifecycle};
use crate::store::Store;

const CASES: &str = include_str!("../../../src/lib/ipc/rule-cases/transcript.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    thread_id: String,
    seq: i64,
    kind: String,
    payload: String,
    created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cases {
    rows: Vec<Row>,
    activity: Value,
    conversation_at: Value,
    conversations: Value,
    resume: Value,
    unknown_conversations: Vec<String>,
}

#[test]
fn every_saved_event_type_is_filed_under_the_kind_the_harness_is_told() {
    let samples = [
        json!({"type": "step", "id": "s", "title": "Stopped", "detail": null, "step": {"kind": "agentStopped"}}),
        json!({"type": "toolCall", "id": "c", "title": "Read", "role": {"kind": "read"}, "status": "pending", "locations": [], "detail": null}),
        json!({"type": "messageChunk", "role": "user", "text": "hi"}),
        json!({"type": "messageChunk", "role": "agent", "text": "hi"}),
        json!({"type": "briefing", "role": "user", "text": "review"}),
        json!({"type": "sessionStarted", "session_id": "s"}),
        json!({"type": "plan", "entries": []}),
        json!({"type": "permission", "id": "p", "tool_call_id": null, "card": crate::model::ApprovalPayload::titled("Use Bash"), "tool": "Bash", "role": {"kind": "read"}, "detail": null, "auto_approved": false}),
        json!({"type": "toolCallUpdate", "id": "c", "title": null, "status": null, "locations": []}),
        json!({"type": "currentTool", "id": null, "title": "Read"}),
        json!({"type": "usage", "used_tokens": 1, "context_size": 2, "cost_usd": null}),
        json!({"type": "turnEnd", "stop_reason": "end_turn", "error": null}),
        json!({"type": "engineExited"}),
    ];
    let mut seen = std::collections::HashSet::new();
    for sample in samples {
        let event_type = sample["type"].as_str().unwrap().to_string();
        let event: super::TranscriptEvent = serde_json::from_value(sample).unwrap();
        let listed = super::KINDS
            .iter()
            .find(|(listed_type, _)| *listed_type == event_type)
            .map(|(_, kind)| *kind);
        assert_eq!(event.kind(), listed, "{event_type}");
        seen.insert(event_type);
    }
    assert_eq!(seen.len(), super::KINDS.len());
}

#[test]
fn the_harness_cases_are_what_the_transcript_reads() {
    let cases: Cases = serde_json::from_str(CASES).unwrap();
    let store = Store::in_memory().unwrap();
    store.seed_workspace("w1", WorkspaceLifecycle::Idle);
    let mut threads: Vec<&str> = cases
        .rows
        .iter()
        .map(|row| row.thread_id.as_str())
        .collect();
    threads.dedup();
    for thread in &threads {
        store.seed_thread(thread, "w1", ThreadStatus::Idle);
    }
    store
        .with_conn(|conn| {
            for row in &cases.rows {
                conn.execute(
                    "INSERT INTO thread_events (thread_id, seq, kind, payload, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        row.thread_id,
                        row.seq,
                        row.kind,
                        row.payload,
                        row.created_at
                    ],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let transcript = Transcript::new(store.clone());

    let mut activity = store.with_read(super::activity_in).unwrap();
    activity.sort_by(|a, b| a.thread_id.cmp(&b.thread_id));
    let mut conversation_at = serde_json::Map::new();
    let mut conversations = serde_json::Map::new();
    for thread in &threads {
        let page = transcript.window(thread, None, None, None).unwrap();
        let at: serde_json::Map<String, Value> = page
            .events
            .iter()
            .map(|event| (event.seq.to_string(), json!(event.conversation)))
            .collect();
        conversation_at.insert(thread.to_string(), Value::Object(at));
        let listed = transcript.conversations(thread).unwrap();
        conversations.insert(thread.to_string(), serde_json::to_value(listed).unwrap());
    }
    assert_eq!(serde_json::to_value(&activity).unwrap(), cases.activity);
    assert_eq!(Value::Object(conversation_at), cases.conversation_at);
    assert_eq!(Value::Object(conversations), cases.conversations);

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Resume {
        thread_id: String,
        id: String,
        session: Option<String>,
    }
    let resume: Vec<Resume> = serde_json::from_value(cases.resume).unwrap();
    assert!(!resume.is_empty());
    for case in resume {
        assert_eq!(
            transcript
                .session_to_resume(&case.thread_id, &case.id)
                .unwrap(),
            case.session,
            "{}",
            case.id
        );
    }
    for id in cases.unknown_conversations {
        assert!(
            transcript.session_to_resume(threads[0], &id).is_err(),
            "{id}"
        );
    }
}
