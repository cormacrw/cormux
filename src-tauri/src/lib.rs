#![allow(dead_code)] // domain APIs fill in as features land

mod approvals;
mod engines;
mod feedback;
mod error;
mod git;
mod github;
mod ipc;
mod llm;
mod mcp;
mod menu;
mod metrics;
mod process;
mod provisioning;
mod shell_env;
mod state;
mod store;
mod summaries;
mod teardown;
mod workspace;

use std::time::Duration;

use ipc::events::StateChanged;
use ipc::types::StateChangeKind;
use state::AppState;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_specta::Event;

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
            if let WindowEvent::Focused(true) = event {
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    match state.fetch.tick().await {
                        Ok(_) => {
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

            let state = AppState::new();
            state.diffs.start();
            state.fetch.spawn_loop();

            let pr_sync = state.pr_sync.clone();
            let pr_app = app.handle().clone();
            let initial_sync = pr_sync.clone();
            pr_sync.spawn_loop(pr_app.clone());
            tauri::async_runtime::spawn(async move {
                if let Err(error) = initial_sync.sync_app(&pr_app).await {
                    log::warn!("initial PR sync failed: {error}");
                }
            });

            let env = state.shell_env.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = env.write().await.load_or_inherit().await {
                    log::warn!("shell env load failed: {error}");
                }
            });

            let db_path = app.path().app_data_dir()?.join("cormux.db");
            state.store.open(&db_path)?;
            app.manage(state);

            let metrics_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(4));
                loop {
                    interval.tick().await;
                    let state = metrics_handle.state::<AppState>();
                    let trees: Vec<(String, Vec<u32>)> = {
                        let mut grouped = std::collections::HashMap::<String, Vec<u32>>::new();
                        for (id, pid) in state.process.pids() {
                            grouped.entry(id).or_default().push(pid);
                        }
                        grouped.into_iter().collect()
                    };
                    if state.metrics.sample(&trees).is_ok() {
                        let version = state.bump_event_version();
                        let _ = StateChanged {
                            version,
                            kind: StateChangeKind::Metrics,
                        }
                        .emit(&metrics_handle);
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
