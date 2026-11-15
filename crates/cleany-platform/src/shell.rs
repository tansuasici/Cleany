//! Handing a path to the OS: show it in the file manager, or open it with
//! its default app.
//!
//! The command builders are pure so every OS's command line is tested on
//! every OS; [`reveal`] and [`open`] run them.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use crate::Os;

/// One argument of a [`Launch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// Passed as is; the platform quotes it.
    Plain(OsString),
    /// Passed verbatim with no quoting added (Windows only: `explorer.exe`
    /// parses its own command line and rejects standard quoting).
    Raw(OsString),
}

/// A program to run and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The program, looked up on `PATH`.
    pub program: &'static str,
    /// Its arguments.
    pub args: Vec<Arg>,
    /// Whether a non-zero exit status means failure. `explorer.exe` exits 1
    /// even when it worked, so its status is not checked.
    pub check_status: bool,
}

impl Launch {
    fn new(program: &'static str, args: Vec<Arg>) -> Self {
        Self {
            program,
            args,
            check_status: true,
        }
    }
}

fn plain(s: impl AsRef<OsStr>) -> Arg {
    Arg::Plain(s.as_ref().to_os_string())
}

/// Why a path could not be handed to the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellError {
    /// Nothing exists at the path any more.
    Gone,
    /// This OS has no known file manager command.
    Unsupported,
    /// Every command failed to start or exited with an error.
    Launch(String),
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gone => f.write_str("the item no longer exists"),
            Self::Unsupported => f.write_str("not supported on this OS"),
            Self::Launch(message) => write!(f, "cannot launch: {message}"),
        }
    }
}

impl std::error::Error for ShellError {}

/// Commands that show `path` selected in the file manager, tried in order
/// until one works. Empty on an unsupported OS.
#[must_use]
pub fn reveal_commands(os: Os, path: &Path) -> Vec<Launch> {
    match os {
        Os::Macos => vec![Launch::new("open", vec![plain("-R"), plain(path)])],
        Os::Windows => {
            // Windows paths cannot contain `"`, so wrapping in quotes is safe;
            // the quotes keep commas and spaces inside the one argument.
            let mut arg = OsString::from("/select,\"");
            arg.push(path);
            arg.push("\"");
            vec![Launch {
                check_status: false,
                ..Launch::new("explorer.exe", vec![Arg::Raw(arg)])
            }]
        }
        Os::Linux => {
            // The FileManager1 D-Bus interface selects the item (Nautilus,
            // Dolphin, Nemo, Thunar…); without it, open the parent folder.
            let show = Launch::new(
                "dbus-send",
                vec![
                    plain("--session"),
                    plain("--print-reply"),
                    plain("--dest=org.freedesktop.FileManager1"),
                    plain("--type=method_call"),
                    plain("/org/freedesktop/FileManager1"),
                    plain("org.freedesktop.FileManager1.ShowItems"),
                    plain(format!("array:string:{}", file_uri(path))),
                    plain("string:"),
                ],
            );
            let folder = path.parent().unwrap_or(path);
            vec![show, Launch::new("xdg-open", vec![plain(folder)])]
        }
        Os::Other => Vec::new(),
    }
}

/// The command that opens `path` with its default app (a folder opens in the
/// file manager). `None` on an unsupported OS.
#[must_use]
pub fn open_command(os: Os, path: &Path) -> Option<Launch> {
    match os {
        Os::Macos => Some(Launch::new("open", vec![plain(path)])),
        Os::Windows => {
            let mut arg = OsString::from("\"");
            arg.push(path);
            arg.push("\"");
            Some(Launch {
                check_status: false,
                ..Launch::new("explorer.exe", vec![Arg::Raw(arg)])
            })
        }
        Os::Linux => Some(Launch::new("xdg-open", vec![plain(path)])),
        Os::Other => None,
    }
}

/// A `file://` URI for `path`, percent-encoding every byte outside the
/// unreserved set and `/`. Commas must be encoded: `dbus-send` splits
/// `array:string:` values on them.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &b in path.as_os_str().as_encoded_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') {
            uri.push(char::from(b));
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

/// Shows `path` selected in the OS file manager.
///
/// # Errors
/// [`ShellError::Gone`] if nothing is at `path`, [`ShellError::Unsupported`]
/// on an unknown OS, [`ShellError::Launch`] if every command failed.
pub fn reveal(path: &Path) -> Result<(), ShellError> {
    ensure_exists(path)?;
    run_first(reveal_commands(Os::current(), path))
}

/// Opens `path` with its default app, as a double-click in the file manager
/// would.
///
/// # Errors
/// As for [`reveal`].
pub fn open(path: &Path) -> Result<(), ShellError> {
    ensure_exists(path)?;
    run_first(open_command(Os::current(), path).into_iter().collect())
}

fn ensure_exists(path: &Path) -> Result<(), ShellError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Err(ShellError::Gone),
        // Other errors (permissions) are the file manager's to report.
        _ => Ok(()),
    }
}

fn run_first(launches: Vec<Launch>) -> Result<(), ShellError> {
    let mut last = ShellError::Unsupported;
    for launch in launches {
        match run(&launch) {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn run(launch: &Launch) -> Result<(), ShellError> {
    let mut cmd = Command::new(launch.program);
    for arg in &launch.args {
        match arg {
            Arg::Plain(s) => {
                cmd.arg(s);
            }
            #[cfg(windows)]
            Arg::Raw(s) => {
                use std::os::windows::process::CommandExt;
                cmd.raw_arg(s);
            }
            #[cfg(not(windows))]
            Arg::Raw(s) => {
                cmd.arg(s);
            }
        }
    }
    let fail = |e: std::io::Error| ShellError::Launch(format!("{}: {e}", launch.program));
    if launch.check_status {
        let status = cmd.status().map_err(fail)?;
        if status.success() {
            Ok(())
        } else {
            Err(ShellError::Launch(format!("{}: {status}", launch.program)))
        }
    } else {
        // Dropping the child does not kill it; Windows leaves no zombie.
        cmd.spawn().map(drop).map_err(fail)
    }
}

#[cfg(test)]
mod tests {
    use super::{Arg, Launch, open_command, reveal_commands};
    use crate::Os;
    use std::ffi::OsString;
    use std::path::Path;

    fn p(s: &str) -> Arg {
        Arg::Plain(OsString::from(s))
    }

    #[test]
    fn reveal_command_macos_selects_in_finder() {
        let got = reveal_commands(Os::Macos, Path::new("/Users/a/b c.txt"));
        assert_eq!(
            got,
            vec![Launch {
                program: "open",
                args: vec![p("-R"), p("/Users/a/b c.txt")],
                check_status: true,
            }]
        );
    }

    #[test]
    fn reveal_command_windows_quotes_select_target() {
        let got = reveal_commands(Os::Windows, Path::new(r"C:\Users\a, b\c d.txt"));
        assert_eq!(
            got,
            vec![Launch {
                program: "explorer.exe",
                args: vec![Arg::Raw(OsString::from(
                    r#"/select,"C:\Users\a, b\c d.txt""#
                ))],
                check_status: false,
            }]
        );
    }

    #[test]
    fn reveal_command_linux_asks_file_manager_with_encoded_uri() {
        let got = reveal_commands(Os::Linux, Path::new("/home/a/x, y%.txt"));
        assert_eq!(got[0].program, "dbus-send");
        assert!(
            got[0]
                .args
                .contains(&p("array:string:file:///home/a/x%2C%20y%25.txt"))
        );
    }

    #[test]
    fn linux_reveal_falls_back_to_parent() {
        let got = reveal_commands(Os::Linux, Path::new("/home/a/x.txt"));
        assert_eq!(got.len(), 2);
        assert_eq!(got[1].program, "xdg-open");
        assert_eq!(got[1].args, vec![p("/home/a")]);
        // The root has no parent; open it itself.
        let root = reveal_commands(Os::Linux, Path::new("/"));
        assert_eq!(root[1].args, vec![p("/")]);
    }

    #[test]
    fn reveal_command_other_is_empty() {
        assert!(reveal_commands(Os::Other, Path::new("/x")).is_empty());
    }

    #[test]
    fn open_command_per_os() {
        let mac = open_command(Os::Macos, Path::new("/a b"));
        assert_eq!(
            mac.map(|l| (l.program, l.args)),
            Some(("open", vec![p("/a b")]))
        );
        let linux = open_command(Os::Linux, Path::new("/a b"));
        assert_eq!(
            linux.map(|l| (l.program, l.args)),
            Some(("xdg-open", vec![p("/a b")]))
        );
        let win = open_command(Os::Windows, Path::new(r"C:\a, b"));
        assert_eq!(
            win.map(|l| (l.program, l.args, l.check_status)),
            Some((
                "explorer.exe",
                vec![Arg::Raw(OsString::from(r#""C:\a, b""#))],
                false
            ))
        );
        assert_eq!(open_command(Os::Other, Path::new("/a")), None);
    }

    #[test]
    fn missing_path_is_gone() {
        let missing = std::env::temp_dir().join("cleany-shell-test-does-not-exist");
        assert_eq!(super::reveal(&missing), Err(super::ShellError::Gone));
        assert_eq!(super::open(&missing), Err(super::ShellError::Gone));
    }
}
