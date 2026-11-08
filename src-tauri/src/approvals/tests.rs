use std::collections::BTreeMap;

use tokio::sync::oneshot;

use super::*;
use crate::engines::ToolRole;
use crate::model::{AgentQuestion, ThreadRole};
use crate::store::types::ThreadRow;

struct Setup {
    broker: ApprovalBroker,
    store: Store,
    events: tokio::sync::broadcast::Receiver<DomainEvent>,
}

fn setup() -> Setup {
    let store = Store::in_memory().unwrap();
    for thread in ["t1", "t2"] {
        store
            .upsert_thread(&ThreadRow::new(thread, "w1", ThreadRole::Lead, "claude"))
            .unwrap();
    }
    let bus = Bus::default();
    let events = bus.subscribe();
    let broker = ApprovalBroker::new(store.clone(), Transcript::new(store.clone()), bus);
    Setup {
        broker,
        store,
        events,
    }
}

fn command(id: &str) -> PermissionRequest {
    PermissionRequest {
        id: id.into(),
        tool_call_id: Some(format!("toolu-{id}")),
        tool: "Bash".into(),
        role: ToolRole::Command {
            command: "ls".into(),
            description: None,
        },
        detail: Some("ls".into()),
        questions: Vec::new(),
        respond: oneshot::channel().0,
    }
}

fn question(id: &str) -> PermissionRequest {
    PermissionRequest {
        questions: vec![AgentQuestion {
            question: "Which colour?".into(),
            header: String::new(),
            options: Vec::new(),
            multi_select: false,
        }],
        ..command(id)
    }
}

fn status(store: &Store, id: &str) -> &'static str {
    store.get_approval(id).unwrap().unwrap().status.as_str()
}

#[tokio::test]
async fn an_answer_reaches_the_waiting_engine_and_the_card_together() {
    let mut setup = setup();
    let decided = setup
        .broker
        .request("t1", 1, &command("a1"), ApprovalPolicy::default());

    let row = setup.store.get_approval("a1").unwrap().unwrap();
    assert_eq!(
        (row.status.as_str(), row.tool.as_str()),
        ("pending", "execute")
    );
    assert_eq!(row.payload.unwrap().title, "Use Bash");
    assert_eq!(
        setup.events.try_recv().unwrap(),
        DomainEvent::ApprovalRequested {
            thread_id: "t1".into(),
            approval_id: "a1".into(),
            tool: "execute".into(),
        }
    );
    assert_eq!(setup.broker.pending_count(), 1);

    setup.broker.resolve("a1", Answer::Approve).unwrap();

    assert_eq!(decided.await.unwrap(), ApprovalDecision::Approved);
    let row = setup.store.get_approval("a1").unwrap().unwrap();
    assert_eq!(row.status.as_str(), "approved");
    assert!(row.payload.unwrap().resolved_at_ms.is_some());
    assert_eq!(setup.broker.pending_count(), 0);
    assert!(setup.broker.resolve("a1", Answer::Approve).is_err());
}

#[tokio::test]
async fn a_denial_tells_the_engine_the_users_reason_or_a_plain_no() {
    let setup = setup();
    let with_reason = setup
        .broker
        .request("t1", 1, &command("a1"), ApprovalPolicy::default());
    let without = setup
        .broker
        .request("t1", 1, &command("a2"), ApprovalPolicy::default());

    let reason = Some("use the other file".to_string());
    setup.broker.resolve("a1", Answer::Deny { reason }).unwrap();
    let blank = Some("  ".to_string());
    setup
        .broker
        .resolve("a2", Answer::Deny { reason: blank })
        .unwrap();

    assert_eq!(
        with_reason.await.unwrap(),
        ApprovalDecision::Denied {
            message: "use the other file".into()
        }
    );
    assert_eq!(
        without.await.unwrap(),
        ApprovalDecision::Denied {
            message: "Denied by user".into()
        }
    );
    let card = |id| {
        setup
            .store
            .get_approval(id)
            .unwrap()
            .unwrap()
            .payload
            .unwrap()
    };
    assert_eq!(
        card("a1").deny_reason.as_deref(),
        Some("use the other file")
    );
    assert_eq!(card("a2").deny_reason, None);
}

#[tokio::test]
async fn a_question_gets_its_answers() {
    let setup = setup();
    let decided = setup
        .broker
        .request("t1", 1, &question("q1"), ApprovalPolicy::default());
    assert_eq!(
        setup.store.get_approval("q1").unwrap().unwrap().tool,
        QUESTION_TOOL
    );

    let answers = BTreeMap::from([("Which colour?".to_string(), "Green".to_string())]);
    setup
        .broker
        .resolve("q1", Answer::Answers(answers.clone()))
        .unwrap();

    assert_eq!(
        decided.await.unwrap(),
        ApprovalDecision::Answered {
            answers: answers.clone()
        }
    );
    let row = setup.store.get_approval("q1").unwrap().unwrap();
    assert_eq!(row.status.as_str(), "approved");
    assert_eq!(row.payload.unwrap().answers, answers);
}

#[tokio::test]
async fn a_turn_ending_cancels_only_that_engines_requests_and_their_cards() {
    let setup = setup();
    let first = setup
        .broker
        .request("t1", 1, &command("a1"), ApprovalPolicy::default());
    let other = setup
        .broker
        .request("t2", 2, &command("b1"), ApprovalPolicy::default());
    let replacement = setup
        .broker
        .request("t1", 3, &command("c1"), ApprovalPolicy::default());

    setup.broker.cancel_for_engine(1);

    assert!(first.await.is_err(), "the waiter is dropped");
    assert_eq!(status(&setup.store, "a1"), "denied");
    assert!(setup.broker.resolve("a1", Answer::Approve).is_err());
    assert_eq!(status(&setup.store, "b1"), "pending");
    setup.broker.resolve("b1", Answer::Approve).unwrap();
    assert_eq!(other.await.unwrap(), ApprovalDecision::Approved);
    assert_eq!(
        status(&setup.store, "c1"),
        "pending",
        "the engine that replaced it on the same thread still asks"
    );
    setup.broker.resolve("c1", Answer::Approve).unwrap();
    assert_eq!(replacement.await.unwrap(), ApprovalDecision::Approved);
}

/// Bug: cancelled cards were denied silently, so after a pause the card still offered Approve
/// and every badge still counted it until something else refreshed the webview.
#[tokio::test]
async fn cancelled_cards_are_announced_once_per_thread_and_only_when_some_were() {
    let mut setup = setup();
    let _waiting: Vec<_> = ["a1", "a2"]
        .into_iter()
        .map(|id| {
            setup
                .broker
                .request("t1", 1, &command(id), ApprovalPolicy::default())
        })
        .collect();
    while setup.events.try_recv().is_ok() {}

    setup.broker.cancel_for_engine(2);
    assert!(setup.events.try_recv().is_err(), "nothing was cancelled");
    setup.broker.cancel_for_engine(1);

    assert_eq!(
        setup.events.try_recv().unwrap(),
        DomainEvent::ApprovalsCancelled {
            thread_id: "t1".into()
        }
    );
    assert!(setup.events.try_recv().is_err());
}

#[tokio::test]
async fn an_answer_nothing_waits_for_is_refused_and_the_card_stops_asking() {
    let setup = setup();
    setup
        .store
        .insert_pending_approval(
            "a1",
            "t1",
            "execute",
            &crate::model::ApprovalPayload::titled("Use Bash"),
        )
        .unwrap();

    assert!(setup.broker.resolve("a1", Answer::Approve).is_err());

    assert_eq!(status(&setup.store, "a1"), "denied", "not approved");
    assert_eq!(setup.broker.pending_count(), 0);
}

#[tokio::test]
async fn an_unreadable_card_is_kept_when_answered() {
    let setup = setup();
    let decided = setup
        .broker
        .request("t1", 1, &command("a1"), ApprovalPolicy::default());
    setup
        .store
        .set_approval_payload_text("a1", "not json")
        .unwrap();
    assert!(
        setup
            .store
            .get_approval("a1")
            .unwrap()
            .unwrap()
            .payload
            .is_none()
    );

    setup.broker.resolve("a1", Answer::Approve).unwrap();

    assert_eq!(decided.await.unwrap(), ApprovalDecision::Approved);
    assert_eq!(status(&setup.store, "a1"), "approved");
    assert_eq!(
        setup.store.approval_payload_text("a1").unwrap().as_deref(),
        Some("not json")
    );
}

#[tokio::test]
async fn running_everything_lets_tools_through_but_questions_still_ask() {
    let setup = setup();
    let policy = ApprovalPolicy::from(&Settings {
        run_everything: true,
        ..Settings::default()
    });

    let tool = setup.broker.request("t1", 1, &command("a1"), policy);
    let asked = setup.broker.request("t1", 1, &question("q1"), policy);

    assert_eq!(tool.await.unwrap(), ApprovalDecision::Approved);
    assert!(setup.store.get_approval("a1").unwrap().is_none(), "no card");
    assert_eq!(status(&setup.store, "q1"), "pending");
    setup.broker.cancel_for_engine(1);
    assert!(asked.await.is_err());
}

/// Bug: a card that couldn't be saved left the agent waiting on a request nobody could see.
#[tokio::test]
async fn a_request_whose_card_cannot_be_saved_is_denied_at_once() {
    let mut setup = setup();
    setup
        .store
        .with_conn(|conn| Ok(conn.execute_batch("DROP TABLE approvals")?))
        .unwrap();

    let decided = setup
        .broker
        .request("t1", 1, &command("a1"), ApprovalPolicy::default());

    let answer = tokio::time::timeout(std::time::Duration::from_secs(5), decided);
    let ApprovalDecision::Denied { message } = answer.await.expect("still waiting").unwrap() else {
        panic!("not denied");
    };
    assert!(message.contains("couldn't record"), "{message}");
    assert!(setup.events.try_recv().is_err(), "no card was announced");
}
