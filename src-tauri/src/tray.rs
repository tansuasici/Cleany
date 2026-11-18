//! Running in the background: the menu bar icon, hiding the window instead of
//! quitting, and starting at login.

use std::fmt;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// The argument the login item starts the app with; the window stays hidden.
pub const HIDDEN_ARG: &str = "--hidden";

/// The window the tray shows and the close button hides.
pub const MAIN_WINDOW: &str = "main";

/// How often the free space in the tray menu is refreshed.
const REFRESH: Duration = Duration::from_secs(60);

/// Language of the tray menu; follows the UI's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Locale {
    /// Turkish.
    #[default]
    Tr,
    /// English.
    En,
}

/// The tray menu items whose text changes after start.
pub struct Tray<R: Runtime> {
    free: MenuItem<R>,
    show: MenuItem<R>,
    quit: MenuItem<R>,
    locale: Mutex<Locale>,
}

impl<R: Runtime> Tray<R> {
    fn locale(&self) -> Locale {
        *self.locale.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Rewrites every label in `locale` and refreshes the free space.
    fn relabel(&self, locale: Locale) {
        *self.locale.lock().unwrap_or_else(PoisonError::into_inner) = locale;
        let (show, quit) = match locale {
            Locale::Tr => ("Cleany'yi göster", "Cleany'den çık"),
            Locale::En => ("Show Cleany", "Quit Cleany"),
        };
        report(self.show.set_text(show));
        report(self.quit.set_text(quit));
        self.refresh();
    }

    /// Puts the startup disk's current free space into the menu.
    fn refresh(&self) {
        report(self.free.set_text(free_text(self.locale(), root_free())));
    }
}

/// Starting at login could not be read or changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LoginItemError {
    /// The OS refused the change.
    Failed {
        /// What went wrong, for the log; the UI shows its own text.
        message: String,
    },
}

impl fmt::Display for LoginItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed { message } => write!(f, "login item: {message}"),
        }
    }
}

impl std::error::Error for LoginItemError {}

/// Adds the menu bar icon and starts refreshing its free space.
///
/// # Errors
/// When the icon or its menu cannot be created.
pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let free = MenuItem::with_id(app, "free", "", false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&free, &separator, &show, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!(
            "../icons/tray-template.png"
        ))?)
        .icon_as_template(true)
        .tooltip("Cleany")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    let tray = Tray {
        free,
        show,
        quit,
        locale: Mutex::new(Locale::default()),
    };
    tray.relabel(Locale::default());
    app.manage(tray);

    let handle = app.clone();
    std::thread::Builder::new()
        .name("cleany-tray".into())
        .spawn(move || {
            loop {
                std::thread::sleep(REFRESH);
                if let Some(tray) = handle.try_state::<Tray<R>>() {
                    tray.refresh();
                }
            }
        })
        .map_err(tauri::Error::Io)?;
    Ok(())
}

/// Brings the main window back and focuses it.
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        report(window.show());
        report(window.unminimize());
        report(window.set_focus());
    }
}

/// Relabels the tray menu in the UI's language.
pub fn set_locale<R: Runtime>(app: &AppHandle<R>, locale: Locale) {
    if let Some(tray) = app.try_state::<Tray<R>>() {
        tray.relabel(locale);
    }
}

/// Whether the app starts at login.
///
/// # Errors
/// [`LoginItemError::Failed`] when the OS setting cannot be read.
pub fn starts_at_login<R: Runtime>(app: &AppHandle<R>) -> Result<bool, LoginItemError> {
    #[cfg(target_os = "macos")]
    if crate::login_item::supported() {
        return Ok(crate::login_item::is_enabled());
    }
    app.autolaunch().is_enabled().map_err(failed)
}

/// Turns starting at login on or off and returns the new state.
///
/// # Errors
/// [`LoginItemError::Failed`] when the OS refuses the change.
pub fn set_start_at_login<R: Runtime>(
    app: &AppHandle<R>,
    on: bool,
) -> Result<bool, LoginItemError> {
    #[cfg(target_os = "macos")]
    if crate::login_item::supported() {
        crate::login_item::set_enabled(on).map_err(failed)?;
        return Ok(crate::login_item::is_enabled());
    }
    let manager = app.autolaunch();
    if on {
        manager.enable().map_err(failed)?;
    } else {
        manager.disable().map_err(failed)?;
    }
    manager.is_enabled().map_err(failed)
}

fn failed(e: impl fmt::Display) -> LoginItemError {
    LoginItemError::Failed {
        message: e.to_string(),
    }
}

fn report(result: tauri::Result<()>) {
    if let Err(e) = result {
        eprintln!("cleany: tray: {e}");
    }
}

/// Free bytes on the volume mounted at `/`, if it is listed.
fn root_free() -> Option<u64> {
    cleany_platform::list_volumes()
        .into_iter()
        .find(|v| v.mount.as_os_str() == "/")
        .map(|v| v.available)
}

/// The tray's free-space line, in the UI's decimal units.
fn free_text(locale: Locale, free: Option<u64>) -> String {
    match (locale, free) {
        (Locale::Tr, Some(n)) => format!("{} boş", format_bytes(n, locale)),
        (Locale::En, Some(n)) => format!("{} free", format_bytes(n, locale)),
        (Locale::Tr, None) => "Boş alan okunamadı".to_owned(),
        (Locale::En, None) => "Free space unavailable".to_owned(),
    }
}

/// Mirrors `formatBytes` in `ui/src/lib/format.ts`: decimal units, one
/// fraction digit below 100 and none above, the locale's decimal mark.
fn format_bytes(bytes: u64, locale: Locale) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    #[allow(clippy::cast_precision_loss)] // Display only; a few digits survive.
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let digits = if unit == 0 || value >= 100.0 { 0 } else { 1 };
    let number = format!("{value:.digits$}");
    let number = match locale {
        Locale::Tr => number.replace('.', ","),
        Locale::En => number,
    };
    format!("{number} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::{Locale, format_bytes, free_text};

    #[test]
    fn bytes_match_the_ui_format() {
        assert_eq!(format_bytes(0, Locale::Tr), "0 B");
        assert_eq!(format_bytes(999, Locale::En), "999 B");
        assert_eq!(format_bytes(48_200_000_000, Locale::Tr), "48,2 GB");
        assert_eq!(format_bytes(48_200_000_000, Locale::En), "48.2 GB");
        assert_eq!(format_bytes(160_400_000_000, Locale::En), "160 GB");
        assert_eq!(format_bytes(1_500_000_000_000, Locale::Tr), "1,5 TB");
    }

    #[test]
    fn free_line_follows_the_locale() {
        assert_eq!(free_text(Locale::Tr, Some(16_000_000_000)), "16,0 GB boş");
        assert_eq!(free_text(Locale::En, Some(16_000_000_000)), "16.0 GB free");
        assert_eq!(free_text(Locale::Tr, None), "Boş alan okunamadı");
    }
}
