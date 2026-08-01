use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, WINDOW_SUBMENU_ID},
    AppHandle, Manager, RunEvent, Runtime, Window, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const SHOW_MAIN_WINDOW_MENU_ID: &str = "show-main-window";
const MAIN_WINDOW_MENU_TITLE: &str = "pH7Console";

/// Extend Tauri's native macOS menu instead of replacing its standard App,
/// File, Edit, View, Window, and Help commands. The Window submenu must use
/// Tauri's reserved identifier so AppKit recognizes it as the application
/// Window menu.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let window_menu = menu
        .get(WINDOW_SUBMENU_ID)
        .and_then(|item| item.as_submenu().cloned())
        .ok_or_else(|| anyhow::anyhow!("Tauri's default Window menu is unavailable"))?;

    window_menu.append(&PredefinedMenuItem::separator(app)?)?;
    window_menu.append(&MenuItem::with_id(
        app,
        SHOW_MAIN_WINDOW_MENU_ID,
        MAIN_WINDOW_MENU_TITLE,
        true,
        None::<&str>,
    )?)?;
    window_menu.append(&PredefinedMenuItem::bring_all_to_front(app, None)?)?;

    Ok(menu)
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    if is_main_window_menu_item(event.id().as_ref()) {
        restore_main_window(app);
    }
}

/// A terminal owns long-lived PTYs and in-memory state, so closing its only
/// window hides it on macOS instead of destroying the workspace while leaving
/// a windowless process running. Command-Q still exits through AppKit's native
/// Quit item.
pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if should_hide_on_close(window.label(), event) {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(error) = window.hide() {
                eprintln!("Failed to hide the pH7Console window: {error}");
            }
        }
    }
}

/// Finder and Dock activation deliver a reopen event to an already-running
/// macOS app. Restore the same retained window even if no window is visible.
pub fn handle_run_event<R: Runtime>(app: &AppHandle<R>, event: &RunEvent) {
    if matches!(event, RunEvent::Reopen { .. }) {
        restore_main_window(app);
    }
}

fn restore_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        eprintln!("The retained pH7Console main window is unavailable");
        return;
    };

    for result in [window.show(), window.unminimize(), window.set_focus()] {
        if let Err(error) = result {
            eprintln!("Failed to restore the pH7Console window: {error}");
            break;
        }
    }
}

fn is_main_window_menu_item(menu_id: &str) -> bool {
    menu_id == SHOW_MAIN_WINDOW_MENU_ID
}

fn should_hide_on_close(window_label: &str, event: &WindowEvent) -> bool {
    window_label == MAIN_WINDOW_LABEL && matches!(event, WindowEvent::CloseRequested { .. })
}

#[cfg(test)]
mod tests {
    use super::{is_main_window_menu_item, MAIN_WINDOW_LABEL, SHOW_MAIN_WINDOW_MENU_ID};

    #[test]
    fn recognizes_only_the_main_window_restore_command() {
        assert!(is_main_window_menu_item(SHOW_MAIN_WINDOW_MENU_ID));
        assert!(!is_main_window_menu_item("quit"));
        assert!(!is_main_window_menu_item("show-settings"));
    }

    #[test]
    fn menu_and_window_identifiers_are_stable() {
        assert_eq!(SHOW_MAIN_WINDOW_MENU_ID, "show-main-window");
        assert_eq!(MAIN_WINDOW_LABEL, "main");
    }
}
