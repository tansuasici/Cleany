//! Tauri shell for Cleany. Commands here stay thin; logic lives in `cleany-core`.

use serde::Serialize;

/// Basic build/runtime info shown in the UI shell. Mirrors `AppInfo` in `ui/src/lib/ipc.ts`.
#[derive(Debug, Serialize)]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "Cleany",
        version: env!("CARGO_PKG_VERSION"),
        os: cleany_platform::Os::current().as_str(),
        arch: std::env::consts::ARCH,
    }
}

/// Builds and runs the Tauri application.
///
/// # Panics
/// Never by design: a startup failure is reported on stderr and the process exits non-zero.
pub fn run() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("cleany: failed to start: {err}");
        std::process::exit(1);
    }
}
