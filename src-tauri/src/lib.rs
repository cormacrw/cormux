#![allow(dead_code)] // domain APIs fill in as features land

mod approvals;
mod engines;
mod error;
mod git;
mod github;
mod ipc;
mod llm;
mod mcp;
mod menu;
mod metrics;
mod process;
mod shell_env;
mod state;
mod store;
mod workspace;

use state::AppState;
use tauri::Manager;

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
            let db_path = app.path().app_data_dir()?.join("cormux.db");
            state.store.open(&db_path)?;
            app.manage(state);

            log::info!("Cormux core started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Cormux");
}
