use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter};

pub const COMMAND_PALETTE: &str = "command-palette";
pub const NEW_WORKSPACE: &str = "new-workspace";
pub const OPEN_SETTINGS: &str = "open-settings";

pub fn build(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let about = PredefinedMenuItem::about(
        app,
        Some("About Cormux"),
        Some(AboutMetadata {
            name: Some("Cormux".into()),
            ..Default::default()
        }),
    )?;
    let settings = MenuItem::with_id(app, OPEN_SETTINGS, "Settings...", true, Some("CmdOrCtrl+,"))?;
    let hide = PredefinedMenuItem::hide(app, None)?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let show_all = PredefinedMenuItem::show_all(app, None)?;
    let quit = PredefinedMenuItem::quit(app, None)?;

    let app_menu = Submenu::with_items(
        app,
        "Cormux",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &settings,
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
    let event = match id {
        COMMAND_PALETTE => "menu://command-palette",
        NEW_WORKSPACE => "menu://new-workspace",
        OPEN_SETTINGS => "menu://open-settings",
        _ => return,
    };
    if let Err(error) = app.emit(event, ()) {
        log::warn!("failed to emit {event}: {error}");
    }
}
