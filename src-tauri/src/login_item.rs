//! Starting at login on macOS 13 and later. The launch agent inside the app
//! bundle is registered through SMAppService, so System Settings and the
//! background activity notice name Cleany instead of the developer. Older
//! macOS keeps the LaunchAgent written by tauri-plugin-autostart.

use std::fs::{self, File};
use std::process::Command;
use std::sync::OnceLock;

use smappservice_rs::{AppService, ServiceStatus, ServiceType};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// The agent in `Contents/Library/LaunchAgents`; it starts Cleany hidden.
const AGENT_PLIST: &str = "com.tansuasici.cleany.login.plist";

/// Held for the life of the process; see [`claim_instance`].
struct InstanceLock(#[allow(dead_code)] File);

/// Whether this macOS has SMAppService (13 and later).
pub fn supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| macos_major().is_some_and(|major| major >= 13))
}

fn macos_major() -> Option<u32> {
    let out = Command::new("/usr/sbin/sysctl")
        .args(["-n", "kern.osproductversion"])
        .output()
        .ok()?;
    String::from_utf8(out.stdout)
        .ok()?
        .trim()
        .split('.')
        .next()?
        .parse()
        .ok()
}

fn agent() -> AppService {
    AppService::new(ServiceType::Agent {
        plist_name: AGENT_PLIST,
    })
}

/// Whether the login agent is registered and allowed in System Settings.
pub fn is_enabled() -> bool {
    agent().status() == ServiceStatus::Enabled
}

/// Registers or removes the login agent.
///
/// # Errors
/// The Service Management error, as text, when macOS refuses the change.
pub fn set_enabled(on: bool) -> Result<(), String> {
    let agent = agent();
    let result = match (on, agent.status()) {
        (true, ServiceStatus::Enabled) | (false, ServiceStatus::NotRegistered) => Ok(()),
        (true, _) => agent.register(),
        (false, _) => agent.unregister(),
    };
    result.map_err(|e| e.to_string())
}

/// Moves a login item left by the plugin's LaunchAgent to the bundled agent.
/// The old one is removed only once the new one is in place, so a failure
/// keeps Cleany starting at login the old way.
pub fn migrate<R: Runtime>(app: &AppHandle<R>) {
    let legacy = app.autolaunch();
    if !legacy.is_enabled().unwrap_or(false) {
        return;
    }
    if let Err(e) = set_enabled(true) {
        eprintln!("cleany: login item: {e}");
        return;
    }
    if let Err(e) = legacy.disable() {
        eprintln!("cleany: login item: {e}");
    }
}

/// Takes the lock that marks a running Cleany; `false` when another process
/// holds it. Registering the agent also starts it once right away, and that
/// hidden copy must not run next to the Cleany that registered it.
pub fn claim_instance<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Ok(dir) = app.path().app_data_dir() else {
        return true;
    };
    let lock = fs::create_dir_all(&dir).and_then(|()| File::create(dir.join("instance.lock")));
    let Ok(file) = lock else {
        return true;
    };
    match file.try_lock() {
        Ok(()) => {
            app.manage(InstanceLock(file));
            true
        }
        Err(std::fs::TryLockError::WouldBlock) => false,
        Err(std::fs::TryLockError::Error(e)) => {
            eprintln!("cleany: instance lock: {e}");
            true
        }
    }
}
