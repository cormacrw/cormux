mod app;
mod approvals;
mod attachments;
mod clickup;
mod composer;
mod create_pr;
mod credentials;
mod engines;
mod error;
mod feedback;
mod findings;
mod findings_block;
mod git;
mod git_workspace;
mod github;
mod harness_config;
mod ipc;
mod llm;
// Not given to any engine yet, so none of its tools are called.
#[allow(dead_code)]
mod mcp;
mod menu;
mod metrics;
mod naming;
mod orchestration;
mod pr_draft;
mod process;
mod provisioning;
mod review;
mod scratch;
mod shell_env;
mod skills;
mod stack;
mod state;
mod store;
mod summaries;
mod teardown;
mod thread_media;
mod window_activity;
mod workspace;

use std::time::Duration;

use ipc::events::{MemorySampled, StateChanged};
use ipc::types::StateChangeKind;
use state::AppState;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_specta::Event;
use window_activity::WindowActivity;

const METRICS_INTERVAL: Duration = Duration::from_secs(4);
/// Nobody reads the sidebar label while the window is in the background, so only every 60s.
const METRICS_BACKGROUND_TICKS: u32 = 15;

/// Dev builds keep their own data, so running Cormux from inside Cormux can't touch the
/// installed app's workspaces.
pub(crate) fn data_dir(app_data: std::path::PathBuf) -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        app_data.join("dev")
    } else {
        app_data
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ipc = ipc::builder();

    #[cfg(debug_assertions)]
    ipc::export_bindings(&ipc);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(ipc.invoke_handler())
        .on_menu_event(|app, event| {
            menu::handle(app, event.id().as_ref());
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(focused) = event {
                let app = window.app_handle().clone();
                let Some(activity) = app.try_state::<WindowActivity>() else {
                    return;
                };
                activity.set_focused(*focused);
                if !*focused || !activity.begin_sync(std::time::Instant::now()) {
                    return;
                }
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    match state.fetch.tick().await {
                        Ok(updates) => {
                            crate::git_workspace::apply_behind_updates(&state, &updates).await;
                            let version = state.bump_event_version();
                            let _ = StateChanged {
                                version,
                                kind: StateChangeKind::BehindCounts,
                            }
                            .emit(&app);
                        }
                        Err(error) => log::warn!("focus fetch failed: {error}"),
                    }
                    let state = app.state::<AppState>();
                    if let Err(error) = state.pr_sync.sync_app(&app).await {
                        log::warn!("focus PR sync failed: {error}");
                    }
                    app.state::<WindowActivity>().finish_sync();
                });
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
            let state = AppState::new();
            state.diffs.start();
            state.fetch.spawn_loop();

            // PR sync shells out to `gh` and `git`, so it waits for the login-shell PATH.
            let env = state.shell_env.clone();
            let pr_sync = state.pr_sync.clone();
            let pr_app = app.handle().clone();
            let startup_app = app.handle().clone();
            let db_path = data_dir(app.path().app_data_dir()?).join("cormux.db");
            state.store.open(&db_path)?;
            state.store.reset_interrupted_turns()?;
            let approval_notify = state.approval_notify.clone();
            let turn_end_notify = state.turn_end_notify.clone();
            state.engines.spawn_idle_reaper();
            app.manage(state);
            // Reads AppState from the handle, so it starts once the state is managed.
            crate::git_workspace::spawn_branch_watch(app.handle().clone());

            tauri::async_runtime::spawn(async move {
                if let Err(error) = env.write().await.load_or_inherit().await {
                    log::warn!("shell env load failed: {error}");
                }
                let startup = startup_app.state::<AppState>();
                startup.process.remove_stale_logs();
                if let Err(error) = crate::git_workspace::restore_fetch_targets(&startup).await {
                    log::warn!("restoring fetch targets failed: {error}");
                }
                if let Err(error) = pr_sync.sync_app(&pr_app).await {
                    log::warn!("initial PR sync failed: {error}");
                }
                pr_sync.spawn_loop(pr_app);
            });

            app.state::<AppState>().mcp.attach_app(app.handle().clone());

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

            let approval_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = approval_notify.subscribe();
                loop {
                    if rx.recv().await.is_err() {
                        break;
                    }
                    let state = approval_app.state::<AppState>();
                    crate::feedback::emit_approval_counts(&approval_app, &state);
                    if let Err(error) =
                        crate::orchestration::on_approval_requested(&approval_app).await
                    {
                        log::warn!("reporting a blocked thread failed: {error}");
                    }
                }
            });

            // The engine pumps only update the database; keep the workspace's live status
            // (what the sidebar shows) in step and tell the UI to refetch.
            let turn_end_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = turn_end_notify.subscribe();
                loop {
                    let thread_id = match rx.recv().await {
                        Ok(thread_id) => thread_id,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    };
                    let state = turn_end_app.state::<AppState>();
                    let _ = state.store.mark_thread_idle(&thread_id);
                    let _ = state.store.deny_stale_approvals(&thread_id);
                    let is_scratch = state
                        .store
                        .scratch_for_thread(&thread_id)
                        .ok()
                        .flatten()
                        .is_some();
                    if let (false, Ok(Some(thread))) =
                        (is_scratch, state.store.thread_by_id(&thread_id))
                    {
                        state
                            .workspace
                            .set_thread(
                                &thread_id,
                                &thread.workspace_id,
                                crate::workspace::ThreadActivity::Idle,
                            )
                            .await;
                    }
                    let version = state.bump_event_version();
                    let _ = StateChanged {
                        version,
                        kind: StateChangeKind::WorkspaceStatus,
                    }
                    .emit(&turn_end_app);
                    if let Err(error) =
                        crate::review::on_reviewer_turn_end(&turn_end_app, &thread_id).await
                    {
                        log::warn!("reading review findings failed: {error}");
                    }
                    if let Err(error) =
                        crate::orchestration::on_turn_end(&turn_end_app, &thread_id).await
                    {
                        log::warn!("reporting a thread failed: {error}");
                    }
                }
            });

            let metrics_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(METRICS_INTERVAL);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                let mut ticks = 0u32;
                loop {
                    interval.tick().await;
                    ticks = ticks.wrapping_add(1);
                    let focused = metrics_handle.state::<WindowActivity>().is_focused();
                    if !focused && !ticks.is_multiple_of(METRICS_BACKGROUND_TICKS) {
                        continue;
                    }
                    let state = metrics_handle.state::<AppState>();
                    if let Ok(sample) = ipc::commands::sample_metrics(&state).await {
                        let _ = MemorySampled { sample }.emit(&metrics_handle);
                    }
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
                if let Err(error) = state.process.stop_all() {
                    log::warn!("stop_all on quit: {error}");
                }
            }
        });
}
