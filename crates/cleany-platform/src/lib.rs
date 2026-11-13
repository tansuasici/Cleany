//! Platform layer: OS detection and raw entry metadata; fast scan paths,
//! trash and reveal come later (M7+).

pub mod meta;

pub use meta::{EntryKind, EntryMeta, read_meta};

use serde::Serialize;

/// The operating system family Cleany is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Windows,
    Linux,
    Other,
}

impl Os {
    /// Returns the OS this binary was compiled for.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }

    /// Short lowercase name, stable for the UI.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Other => "other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Os;

    #[test]
    fn current_matches_target_os() {
        let expected = std::env::consts::OS;
        let got = Os::current().as_str();
        if matches!(expected, "macos" | "windows" | "linux") {
            assert_eq!(got, expected);
        } else {
            assert_eq!(got, "other");
        }
    }
}
