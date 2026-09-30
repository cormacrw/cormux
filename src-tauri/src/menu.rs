use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter};

pub const COMMAND_PALETTE: &str = "command-palette";
pub const HOMEBASE: &str = "homebase";
pub const HIDE: &str = "hide";
pub const NEW_WORKSPACE: &str = "new-workspace";
pub const NEW_SCRATCH: &str = "new-scratch";
pub const OPEN_SETTINGS: &str = "open-settings";
pub const RELOAD_ENVIRONMENT: &str = "reload-environment";

/// Dev builds name themselves so they can't pass for the installed app.
const APP_NAME: &str = if cfg!(debug_assertions) {
    "Cormux Dev"
} else {
    "Cormux"
};

pub fn build(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let about = PredefinedMenuItem::about(
        app,
        Some(&format!("About {APP_NAME}")),
        Some(AboutMetadata {
            name: Some(APP_NAME.into()),
            ..Default::default()
        }),
    )?;
    let settings = MenuItem::with_id(app, OPEN_SETTINGS, "Settings...", true, Some("CmdOrCtrl+,"))?;
    let reload_env = MenuItem::with_id(
        app,
        RELOAD_ENVIRONMENT,
        "Reload environment",
        true,
        None::<&str>,
    )?;
    // Not the predefined Hide: that one owns ⌘H, which goes to Homebase.
    let hide = MenuItem::with_id(app, HIDE, format!("Hide {APP_NAME}"), true, None::<&str>)?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let show_all = PredefinedMenuItem::show_all(app, None)?;
    let quit = PredefinedMenuItem::quit(app, None)?;

    let app_menu = Submenu::with_items(
        app,
        APP_NAME,
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &reload_env,
            &PredefinedMenuItem::separator(app)?,
            &hide,
            &hide_others,
            &show_all,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let new_workspace = MenuItem::with_id(
        app,
        NEW_WORKSPACE,
        "New Workspace",
        true,
        Some("CmdOrCtrl+N"),
    )?;
    let new_scratch =
        MenuItem::with_id(app, NEW_SCRATCH, "New Scratch", true, Some("CmdOrCtrl+S"))?;
    let file_menu = Submenu::with_items(app, "File", true, &[&new_workspace, &new_scratch])?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let command_palette = MenuItem::with_id(
        app,
        COMMAND_PALETTE,
        "Command Palette",
        true,
        Some("CmdOrCtrl+K"),
    )?;
    let homebase = MenuItem::with_id(app, HOMEBASE, "Go to Homebase", true, Some("CmdOrCtrl+H"))?;
    let view_menu = Submenu::with_items(app, "View", true, &[&homebase, &command_palette])?;

    Menu::with_items(app, &[&app_menu, &file_menu, &edit_menu, &view_menu])
}

pub fn handle(app: &AppHandle, id: &str) {
    if id == HIDE {
        #[cfg(target_os = "macos")]
        if let Err(error) = app.hide() {
            log::warn!("failed to hide app: {error}");
        }
        return;
    }
    let event = match id {
        HOMEBASE => "menu://homebase",
        COMMAND_PALETTE => "menu://command-palette",
        NEW_WORKSPACE => "menu://new-workspace",
        NEW_SCRATCH => "menu://new-scratch",
        OPEN_SETTINGS => "menu://open-settings",
        RELOAD_ENVIRONMENT => "menu://reload-environment",
        _ => return,
    };
    if let Err(error) = app.emit(event, ()) {
        log::warn!("failed to emit {event}: {error}");
    }
}
