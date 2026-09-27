#![allow(dead_code)] // domain APIs fill in as features land

mod approvals;
mod engines;
mod error;
mod git;
mod github;
mod ipc;
mod llm;
mod mcp;
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
        .invoke_handler(ipc.invoke_handler())
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
            app.manage(AppState::new());
            log::info!("Harness core started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Harness");
}
