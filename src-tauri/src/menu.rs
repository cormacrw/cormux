use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri_specta::Event;

pub const COMMAND_PALETTE: &str = "command-palette";
pub const HIDE: &str = "hide";
pub const NEW_WORKSPACE: &str = "new-workspace";
pub const OPEN_SETTINGS: &str = "open-settings";
pub const RELOAD_ENVIRONMENT: &str = "reload-environment";

/// A menu item the webview acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum MenuCommand {
    CommandPalette,
    NewWorkspace,
    OpenSettings,
    ReloadEnvironment,
}

/// The user picked `command` from the app's menu.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct MenuChosen {
    pub command: MenuCommand,
}

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
    // Our own item, so the handler can hide the window itself.
    let hide = MenuItem::with_id(
        app,
        HIDE,
        format!("Hide {APP_NAME}"),
        true,
        Some("CmdOrCtrl+H"),
    )?;
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

    let new_workspace =
        MenuItem::with_id(app, NEW_WORKSPACE, "New Thread", true, Some("CmdOrCtrl+N"))?;
    let file_menu = Submenu::with_items(app, "File", true, &[&new_workspace])?;

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
    let view_menu = Submenu::with_items(app, "View", true, &[&command_palette])?;

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
    let command = match id {
        COMMAND_PALETTE => MenuCommand::CommandPalette,
        NEW_WORKSPACE => MenuCommand::NewWorkspace,
        OPEN_SETTINGS => MenuCommand::OpenSettings,
        RELOAD_ENVIRONMENT => MenuCommand::ReloadEnvironment,
        _ => return,
    };
    if let Err(error) = (MenuChosen { command }).emit(app) {
        log::warn!("failed to emit menu {command:?}: {error}");
    }
}
