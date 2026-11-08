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
mod findings_block;
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
mod summaries;
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
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_specta::Event;
use window_activity::WindowActivity;

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
                    workspace_git::fetch_all(&state).await;
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
            )?;
            state.diffs.start();
            state.engines.spawn_idle_reaper();
            app.manage(state.clone());

            feedback::start(&state.bus, &state.out);
            host_tools::start(&state);
            turn::start_settling(&state);
            orchestration::start(&state);
            review::start(&state);
            let focus = app.handle().clone();
            let events = app.handle().clone();
            metrics::start(
                &state,
                move || focus.state::<WindowActivity>().is_focused(),
                move |sample| {
                    let _ = MemorySampled { sample }.emit(&events);
                },
            );
            // Git and `gh` need the login-shell PATH, so what shells out waits for it.
            let startup = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = startup.shell_env.write().await.load_or_inherit().await {
                    log::warn!("shell env load failed: {error}");
                }
                startup.process.remove_stale_logs();
                workspace_git::start(&startup);
                github::start(&startup);
            });

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
