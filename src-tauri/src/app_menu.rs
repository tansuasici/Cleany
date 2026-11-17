//! The macOS app menu: the standard items plus "Settings…" (Cmd+,), which
//! brings the window forward and opens the Settings dialog.

use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
use tauri::{AppHandle, Manager, Runtime};

use crate::tray::{self, Locale};

const SETTINGS_ID: &str = "settings";

/// The DOM event the UI listens for to open the Settings dialog.
const OPEN_SETTINGS_EVENT: &str = "cleany:open-settings";

/// The app menu item whose text follows the UI's language.
struct AppMenu<R: Runtime> {
    settings: MenuItem<R>,
}

/// Replaces the default menu with one that has "Settings…" under the app menu.
///
/// # Errors
/// When the menu or its items cannot be created.
pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = Menu::default(app)?;
    let settings = MenuItem::with_id(
        app,
        SETTINGS_ID,
        label(Locale::default()),
        true,
        Some("CmdOrCtrl+,"),
    )?;
    // The first submenu is the app menu: About, separator, Services, …
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        app_menu.insert(&settings, 2)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, 3)?;
    }
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if event.id().as_ref() == SETTINGS_ID {
            open_settings(app);
        }
    });
    app.manage(AppMenu { settings });
    Ok(())
}

/// Relabels "Settings…" in the UI's language.
pub fn set_locale<R: Runtime>(app: &AppHandle<R>, locale: Locale) {
    if let Some(menu) = app.try_state::<AppMenu<R>>()
        && let Err(e) = menu.settings.set_text(label(locale))
    {
        eprintln!("cleany: app menu: {e}");
    }
}

fn label(locale: Locale) -> &'static str {
    match locale {
        Locale::Tr => "Ayarlar…",
        Locale::En => "Settings…",
    }
}

fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    tray::show_main(app);
    if let Some(window) = app.get_webview_window(tray::MAIN_WINDOW)
        && let Err(e) = window.eval(format!(
            "window.dispatchEvent(new Event('{OPEN_SETTINGS_EVENT}'))"
        ))
    {
        eprintln!("cleany: open settings: {e}");
    }
}
