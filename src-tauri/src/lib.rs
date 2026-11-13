mod app;
mod approvals;
mod attachments;
mod bus;
mod changes;
mod clickup;
mod create_pr;
mod credentials;
mod engines;
mod entities;
mod environment;
mod error;
mod feedback;
mod findings;
mod git;
mod github;
mod harness_config;
mod host_tools;
mod ipc;
mod llm;
mod media;
mod menu;
mod metrics;
mod model;
mod naming;
mod orchestration;
mod paths;
mod pr_draft;
mod process;
mod process_tree;
mod provisioning;
mod repos;
mod review;
mod scratch;
mod settings;
mod shell_env;
mod skills;
mod snapshot;
mod stack;
mod state;
mod store;
mod teardown;
mod terminal;
mod thread_media;
mod thread_runtime;
mod time;
mod timeline;
mod transcript;
mod turn;
mod window_activity;
mod workspace_git;
mod workspace_start;
mod workspaces;

use std::sync::Arc;
use std::time::Duration;

use ipc::events::{MemorySampled, TauriOutput};
use ipc::subscriptions::Subscriptions;
use state::AppState;
use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_specta::Event;
use window_activity::WindowActivity;

/// What each owner puts back after a quit, in order: owners that read which turns were in flight
/// go before `workspaces::restore_after_quit` ends them.
const ON_RESTART: &[state::OnRestart] = &[
    approvals::on_restart,
    review::on_restart,
    findings::on_restart,
    workspaces::restore_after_quit,
];

/// The features that react to what happens in the core; the bus hands each event to them in
/// `bus::Listener` order.
fn start_listeners(state: &AppState) {
    feedback::start(&state.bus, &state.out, &state.store);
    review::start(state);
    findings::start(state);
    orchestration::start(state);
}

pub fn run() {
    let ipc = ipc::builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(ipc.invoke_handler())
        .on_menu_event(|app, event| {
            menu::handle(app, event.id().as_ref());
        })
        .on_page_load(|webview, payload| {
            // The first load can come before `setup` has managed it, with nothing to cancel.
            if payload.event() == PageLoadEvent::Started
                && let Some(subs) = webview.try_state::<Subscriptions>()
            {
                subs.cancel_all();
            }
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(focused) = event {
                window_activity::focus_changed(window.app_handle(), *focused);
            }
        })
        .setup(move |app| {
            let log_level = if cfg!(debug_assertions) {
                log::LevelFilter::Info
            } else {
                log::LevelFilter::Warn
            };

            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log_level)
                    .build(),
            )?;

            ipc.mount_events(app);
            app.set_menu(menu::build(app.handle())?)?;

            app.manage(WindowActivity::new());
            app.manage(Subscriptions::new());
            let state = AppState::new(
                paths::DataDir::for_app(app.path().app_data_dir()?),
                Arc::new(TauriOutput(app.handle().clone())),
                ON_RESTART,
            )?;
            app.asset_protocol_scope()
                .allow_directory(state.data_dir.media(), true)?;
            state.diffs.start();
            state.engines.spawn_idle_reaper();
            app.manage(state.clone());

            host_tools::start(&state);
            turn::start_settling(&state);
            start_listeners(&state);
            let focus = app.handle().clone();
            let events = app.handle().clone();
            metrics::start(
                &state,
                move || focus.state::<WindowActivity>().is_focused(),
                move |sample| {
                    let _ = MemorySampled { sample }.emit(&events);
                },
            );
            let env = state.shell_env.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = env.load_or_inherit().await {
                    log::warn!("shell env load failed: {error}");
                }
            });
            state.process.remove_stale_logs();
            workspace_git::start(&state);
            github::start(&state);

            // The window starts hidden and the app shows it once it has painted. If
            // that never happens (a startup error), show it anyway rather than
            // leave Cormux running with no window.
            let fallback = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(4)).await;
                if let Some(window) = fallback.get_webview_window("main")
                    && !window.is_visible().unwrap_or(true)
                {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            });

            log::info!("Cormux core started");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Cormux")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                let state = app.state::<AppState>();
                state.engines.kill_all();
                state.process.stop_all();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::{AgentEvent, EngineCommand, MessageRole};
    use crate::model::ApprovalStatus;
    use crate::model::{ThreadRole, ThreadStatus, WorkspaceKind, WorkspaceLifecycle};
    use crate::store::Store;
    use crate::store::fixtures;
    use crate::store::types::{ThreadRow, WorkspaceRow};

    fn restart(store: &Store) {
        for on_restart in ON_RESTART {
            on_restart(store).unwrap();
        }
    }

    #[test]
    fn a_restart_ends_the_turns_that_were_running() {
        let store = Store::in_memory().unwrap();
        store.seed_workspace("w", WorkspaceLifecycle::Paused);
        store
            .with_conn(|conn| {
                conn.execute_batch(
                    "PRAGMA foreign_keys = OFF;
                     INSERT INTO threads (id, workspace_id, title, engine, status) VALUES
                        ('busy', 'w', 'Lead', 'claude', 'running'),
                        ('held', 'w', 'Helper', 'claude', 'paused'),
                        ('setup', 'w2', 'Lead', 'claude', 'provisioning');
                     INSERT INTO approvals (id, thread_id, status, tool, payload) VALUES
                        ('stale', 'busy', 'pending', 'execute', '{}'),
                        ('done', 'busy', 'approved', 'execute', '{}');",
                )?;
                Ok(())
            })
            .unwrap();

        restart(&store);

        let approval = |id: &str| store.get_approval(id).unwrap().unwrap().status;
        assert_eq!(approval("stale"), ApprovalStatus::Denied);
        assert_eq!(approval("done"), ApprovalStatus::Approved);
        let status = |id: &str| store.thread_by_id(id).unwrap().unwrap().status;
        assert_eq!(status("busy"), ThreadStatus::Idle);
        assert_eq!(status("held"), ThreadStatus::Idle);
        assert_eq!(status("setup"), ThreadStatus::Provisioning);
        let workspace = store.workspace_by_id("w").unwrap().unwrap();
        assert_eq!(workspace.status, WorkspaceLifecycle::Idle);
    }

    /// A quit mid-setup used to leave it `provisioning`, which Retry and Skip refuse, and a
    /// quit mid-teardown `tearingDown`, which teardown refuses: stuck for good either way.
    #[test]
    fn a_restart_hands_back_interrupted_setups_and_teardowns() {
        let store = Store::in_memory().unwrap();
        for (id, status) in [
            ("setting-up", WorkspaceLifecycle::Provisioning),
            ("torn-mid-turn", WorkspaceLifecycle::TearingDown),
            ("torn-mid-setup", WorkspaceLifecycle::TearingDown),
            ("torn-empty", WorkspaceLifecycle::TearingDown),
        ] {
            store.seed_workspace(id, status);
        }
        store.seed_thread("lead", "setting-up", ThreadStatus::Provisioning);
        store.seed_thread("busy", "torn-mid-turn", ThreadStatus::Running);
        store.seed_thread("setup", "torn-mid-setup", ThreadStatus::Provisioning);

        restart(&store);

        let status = |id: &str| store.workspace_by_id(id).unwrap().unwrap().status;
        assert_eq!(status("setting-up"), WorkspaceLifecycle::ProvisioningFailed);
        assert_eq!(status("torn-mid-turn"), WorkspaceLifecycle::Idle);
        assert_eq!(
            status("torn-mid-setup"),
            WorkspaceLifecycle::ProvisioningFailed
        );
        assert_eq!(status("torn-empty"), WorkspaceLifecycle::Ready);
    }

    /// Bug: a quit mid branch review left it running for good, so every later Lead turn was
    /// read as its reply.
    #[test]
    fn a_restart_ends_a_branch_review_whose_turn_was_in_flight() {
        let store = Store::in_memory().unwrap();
        store.upsert_repo(&fixtures::repo("r1")).unwrap();
        for (workspace, kind, thread, status) in [
            (
                "reviewing",
                WorkspaceKind::Lead,
                "lead",
                ThreadStatus::Running,
            ),
            (
                "asked-again",
                WorkspaceKind::Lead,
                "waiting",
                ThreadStatus::Idle,
            ),
            (
                "pr",
                WorkspaceKind::PrReview,
                "reviewer",
                ThreadStatus::Running,
            ),
        ] {
            store
                .insert_workspace(&WorkspaceRow {
                    kind,
                    ..fixtures::workspace(workspace, WorkspaceLifecycle::Running)
                })
                .unwrap();
            store.seed_thread(thread, workspace, status);
            store
                .set_review_status(workspace, review::ReviewStatus::Running)
                .unwrap();
        }

        restart(&store);

        let review = |id: &str| store.review(id).unwrap().unwrap().status;
        assert_eq!(review("reviewing"), None);
        assert_eq!(review("asked-again"), Some(review::ReviewStatus::Running));
        assert_eq!(review("pr"), Some(review::ReviewStatus::Running));
    }

    /// Bug: findings held for a thread paused at a quit stayed sent to a message that never
    /// reached it.
    #[test]
    fn a_restart_reopens_findings_a_paused_thread_held() {
        let store = Store::in_memory().unwrap();
        let transcript = transcript::Transcript::new(store.clone());
        store.seed_workspace("w", WorkspaceLifecycle::Paused);
        store.seed_thread("held", "w", ThreadStatus::Paused);
        store.seed_thread("taken", "w", ThreadStatus::Paused);
        let found = |title: &str| findings::ReviewFinding {
            severity: findings::FindingSeverity::Blocking,
            title: title.into(),
            file: None,
            line: None,
            explanation: String::new(),
        };
        findings::record_review(&store, "w", vec![found("a"), found("b")]).unwrap();
        let ids: Vec<String> = store
            .findings_for_workspace("w")
            .unwrap()
            .into_iter()
            .map(|finding| finding.id)
            .collect();
        let send = |thread: &str, id: &String| {
            let seq = transcript.record_user_message(thread, "Fix").unwrap();
            store
                .mark_findings_sent(std::slice::from_ref(id), thread)
                .unwrap();
            store
                .set_findings_batch(
                    thread,
                    Some(&findings::FindingsBatch {
                        ids: vec![id.clone()],
                        message_seq: seq,
                    }),
                )
                .unwrap();
        };
        transcript
            .record_step("held", timeline::TimelineStep::AgentPaused)
            .unwrap();
        send("held", &ids[0]);
        send("taken", &ids[1]);
        transcript
            .record_step("taken", timeline::TimelineStep::AgentPaused)
            .unwrap();

        restart(&store);

        let finding = |id: &String| store.finding_by_id(id).unwrap().unwrap().status;
        assert_eq!(finding(&ids[0]), findings::FindingStatus::Open);
        assert!(store.findings_batch("held").unwrap().is_none());
        assert_eq!(finding(&ids[1]), findings::FindingStatus::Sent);
        assert!(store.findings_batch("taken").unwrap().is_some());
    }

    /// The order the app starts its listeners in is the bus's: a Lead whose branch review ends
    /// in the same turn a child's report waits for it has its findings read before the report
    /// starts its next turn.
    #[tokio::test]
    async fn the_apps_listeners_read_a_turn_before_starting_the_next() {
        let (state, _recording, fake) = AppState::faked(paths::DataDir::new(std::env::temp_dir()));
        state.store.upsert_repo(&fixtures::repo("r1")).unwrap();
        for (workspace, thread, parent) in [("w1", "t1", None), ("w2", "child", Some("t1"))] {
            state
                .store
                .insert_workspace(&WorkspaceRow {
                    worktree_path: std::env::temp_dir().display().to_string(),
                    ..fixtures::workspace(workspace, WorkspaceLifecycle::Running)
                })
                .unwrap();
            state
                .store
                .upsert_thread(&ThreadRow {
                    parent_thread_id: parent.map(str::to_string),
                    ..ThreadRow::new(thread, workspace, ThreadRole::Lead, "claude")
                        .with_status(ThreadStatus::Running)
                })
                .unwrap();
        }
        state
            .store
            .set_review_status("w1", review::ReviewStatus::Running)
            .unwrap();
        let said = |thread: &str, text: &str| {
            state
                .transcript
                .append(
                    thread,
                    AgentEvent::MessageChunk {
                        role: MessageRole::Agent,
                        text: text.into(),
                    },
                )
                .unwrap();
        };
        let mut lead = fake.engine("t1");
        state
            .engines
            .ensure(&state.engines.operation("t1").await, "w1")
            .await
            .unwrap();
        start_listeners(&state);

        said("child", "child done");
        turn::end_turn(&state, "child").await;
        state.bus.settled().await;
        said("t1", "Clean.\n\n<cormux-findings>\n[]\n</cormux-findings>");
        turn::end_turn(&state, "t1").await;

        let prompt = loop {
            match tokio::time::timeout(std::time::Duration::from_secs(5), lead.recv()).await {
                Ok(Some(EngineCommand::Prompt(prompt))) => break prompt.text,
                Ok(Some(_)) => {}
                other => panic!("no report: {other:?}"),
            }
        };
        let review = state.store.review("w1").unwrap().unwrap();
        assert_eq!(review.status, Some(review::ReviewStatus::Ready));
        assert!(prompt.contains("child done"), "{prompt}");
        assert_eq!(
            state.bus.listeners(),
            [
                bus::Listener::Feedback,
                bus::Listener::Review,
                bus::Listener::Findings,
                bus::Listener::Orchestration,
            ]
        );
    }
}
