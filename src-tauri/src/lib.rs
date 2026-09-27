#![allow(dead_code)] // stub APIs; wired to IPC in COR-35

mod approvals;
mod engines;
mod error;
mod git;
mod github;
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
    tauri::Builder::default()
        .setup(|app| {
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

            app.manage(AppState::new());
            log::info!("Harness core started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Harness");
}
