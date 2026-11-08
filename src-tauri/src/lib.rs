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
mod mcp;
mod menu;
mod metrics;
mod model;
mod naming;
mod orchestration;
mod paths;
mod pr_draft;
mod process;
mod provisioning;
mod repos;
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
mod time;
mod timeline;
mod turn;
mod window_activity;
mod workspace;
mod workspace_start;

use std::time::Duration;

use std::sync::Arc;

use feedback::Change;
use ipc::events::{MemorySampled, TauriOutput};
use ipc::subscriptions::Subscriptions;
use state::AppState;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_specta::Event;
use window_activity::WindowActivity;

const METRICS_INTERVAL: Duration = Duration::from_secs(4);
/// Nobody reads the sidebar label while the window is in the background, so only every 60s.
const METRICS_BACKGROUND_TICKS: u32 = 15;

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
                    let updates = state.fetch.tick().await;
                    crate::git_workspace::apply_behind_updates(&state, &updates).await;
                    state.out.changed(Change::REPO_GIT);
                    let state = app.state::<AppState>();
                    if let Err(error) = state.pr_sync.sync_app(&state).await {
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
            app.manage(Subscriptions::new());
            let state = AppState::new(
                paths::DataDir::for_app(app.path().app_data_dir()?),
                Arc::new(TauriOutput(app.handle().clone())),
            );
            state.diffs.start();
            state.fetch.spawn_loop();

            // PR sync shells out to `gh` and `git`, so it waits for the login-shell PATH.
            let env = state.shell_env.clone();
            let pr_sync = state.pr_sync.clone();
            state.store.open(&state.data_dir.db())?;
            state.store.reset_interrupted_turns()?;
            let approval_notify = state.approval_notify.clone();
            state.engines.spawn_idle_reaper();
            app.manage(state.clone());
            crate::git_workspace::spawn_branch_watch(state.clone());

            let startup = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = env.write().await.load_or_inherit().await {
                    log::warn!("shell env load failed: {error}");
                }
                startup.process.remove_stale_logs();
                if let Err(error) = crate::git_workspace::restore_fetch_targets(&startup).await {
                    log::warn!("restoring fetch targets failed: {error}");
                }
                if let Err(error) = pr_sync.sync_app(&startup).await {
                    log::warn!("initial PR sync failed: {error}");
                }
                pr_sync.spawn_loop(startup);
            });

            state.mcp.attach(state.clone());

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

            let approval_state = state.clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = approval_notify.subscribe();
                loop {
                    if rx.recv().await.is_err() {
                        break;
                    }
                    approval_state.out.changed(Change::THREADS);
                    if let Err(error) =
                        crate::orchestration::on_approval_requested(&approval_state).await
                    {
                        log::warn!("reporting a blocked thread failed: {error}");
                    }
                }
            });

            tauri::async_runtime::spawn(turn::listen(state));

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
                    if let Ok(sample) = metrics::sample_processes(&state).await {
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
                state.engines.kill_all();
                if let Err(error) = state.process.stop_all() {
                    log::warn!("stop_all on quit: {error}");
                }
            }
        });
}
