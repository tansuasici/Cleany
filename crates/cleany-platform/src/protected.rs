//! The OS folders Cleany never trashes, per platform, and the check against
//! them. Entries are compared by identity (device + file id), so spelling,
//! letter case, `\\?\` prefixes and symlinked parents do not matter.

use std::path::{Path, PathBuf};

use crate::meta::read_meta;

/// How much of a listed folder is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    /// The folder itself; what is inside may go.
    Itself,
    /// The folder and everything below it.
    Subtree,
}

/// One listed folder, before it is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Rule {
    /// A protected folder.
    Protect(PathBuf, Reach),
    /// A folder under a [`Reach::Subtree`] entry whose contents may go
    /// (`/usr/local` under `/usr`). The folder itself still needs its own
    /// [`Reach::Itself`] rule to be protected.
    // Windows lists no exception yet; the tests still build one there.
    #[cfg_attr(windows, allow(dead_code))]
    Except(PathBuf),
}

/// How a filesystem entry is identified.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    /// Device and file id.
    Id(u64, u64),
    /// Canonical path, where the OS gives no file id (Windows folders).
    Path(PathBuf),
}

/// The key of `path`, without following a final symlink; `None` when it
/// does not exist.
fn key(path: &Path) -> Option<Key> {
    let m = read_meta(path).ok()?;
    if m.file_id != 0 {
        return Some(Key::Id(m.dev, m.file_id));
    }
    path.canonicalize().ok().map(Key::Path)
}

/// The key of `path` with links followed.
fn real_key(path: &Path) -> Option<Key> {
    path.canonicalize().ok().as_deref().and_then(key)
}

/// The rule list resolved to identities; built once per request.
#[derive(Debug, Default)]
pub(crate) struct Protected {
    /// Protected entries, with both the link and its target for a listed
    /// symlink.
    entries: Vec<(Key, Reach)>,
    /// Folders whose contents escape a [`Reach::Subtree`] above them.
    except: Vec<Key>,
    /// Every folder above a protected entry.
    holders: Vec<Key>,
}

impl Protected {
    /// Resolves `rules`; entries that do not exist are skipped.
    pub(crate) fn resolve(rules: &[Rule]) -> Self {
        let mut p = Self::default();
        for rule in rules {
            match rule {
                Rule::Protect(path, reach) => {
                    let (Some(own), real) = (key(path), real_key(path)) else {
                        continue;
                    };
                    if let Some(real) = real.filter(|r| *r != own) {
                        p.entries.push((real, *reach));
                    }
                    p.entries.push((own, *reach));
                    let Ok(canon) = path.canonicalize() else {
                        continue;
                    };
                    for a in canon.ancestors().skip(1) {
                        if let Some(k) = key(a)
                            && !p.holders.contains(&k)
                        {
                            p.holders.push(k);
                        }
                    }
                }
                Rule::Except(path) => {
                    if let Some(k) = real_key(path) {
                        p.except.push(k);
                    }
                }
            }
        }
        p
    }

    /// Whether `path` is a protected entry, lies under a [`Reach::Subtree`]
    /// entry, or holds a protected entry. A symlink is judged as the link:
    /// one pointing at a protected folder is not protected, a listed one is.
    pub(crate) fn covers(&self, path: &Path) -> bool {
        let Some(own) = key(path) else {
            return false;
        };
        if self.entries.iter().any(|(k, _)| *k == own) || self.holders.contains(&own) {
            return true;
        }
        // The parent chain, with links resolved, nearest first.
        let Some(parent) = path.parent().and_then(|p| p.canonicalize().ok()) else {
            return false;
        };
        for a in parent.ancestors() {
            let Some(k) = key(a) else { continue };
            if self.except.contains(&k) {
                return false;
            }
            if self.entries.contains(&(k, Reach::Subtree)) {
                return true;
            }
        }
        false
    }
}

/// The rules for this OS; `home` adds the user folders that must stay.
pub(crate) fn system_rules(home: Option<&Path>) -> Vec<Rule> {
    let mut rules = os_rules();
    if let Some(home) = home {
        for name in HOME_FOLDERS {
            rules.push(Rule::Protect(home.join(name), Reach::Itself));
        }
    }
    rules
}

/// Folders right under the home folder that are kept as folders.
#[cfg(target_os = "macos")]
const HOME_FOLDERS: &[&str] = &[
    "Library",
    "Desktop",
    "Documents",
    "Downloads",
    "Movies",
    "Music",
    "Pictures",
];

/// Folders right under the home folder that are kept as folders.
#[cfg(windows)]
const HOME_FOLDERS: &[&str] = &[
    "AppData",
    "Desktop",
    "Documents",
    "Downloads",
    "Music",
    "Pictures",
    "Videos",
];

/// Folders right under the home folder that are kept as folders.
#[cfg(all(unix, not(target_os = "macos")))]
const HOME_FOLDERS: &[&str] = &[
    ".config",
    ".local",
    "Desktop",
    "Documents",
    "Downloads",
    "Music",
    "Pictures",
    "Videos",
];

/// `path` protected to `reach`.
#[cfg(unix)]
fn protect(path: &str, reach: Reach) -> Rule {
    Rule::Protect(PathBuf::from(path), reach)
}

#[cfg(target_os = "macos")]
fn os_rules() -> Vec<Rule> {
    use Reach::{Itself, Subtree};
    let mut rules: Vec<Rule> = [
        "/System",
        "/bin",
        "/sbin",
        "/usr",
        "/private/etc",
        "/private/var/db",
        "/Library/Apple",
    ]
    .into_iter()
    .map(|p| protect(p, Subtree))
    .collect();
    rules.extend(
        [
            "/Applications",
            "/Library",
            "/Users",
            "/Volumes",
            "/etc",
            "/tmp",
            "/var",
            "/private",
            "/private/var",
            "/private/tmp",
            "/opt",
            "/usr/local",
        ]
        .into_iter()
        .map(|p| protect(p, Itself)),
    );
    rules.push(Rule::Except(PathBuf::from("/usr/local")));
    rules
}

#[cfg(all(unix, not(target_os = "macos")))]
fn os_rules() -> Vec<Rule> {
    use Reach::{Itself, Subtree};
    let mut rules: Vec<Rule> = [
        "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32", "/usr", "/etc", "/boot", "/proc",
        "/sys", "/dev", "/run",
    ]
    .into_iter()
    .map(|p| protect(p, Subtree))
    .collect();
    rules.extend(
        [
            "/home",
            "/root",
            "/var",
            "/opt",
            "/srv",
            "/mnt",
            "/media",
            "/tmp",
            "/usr/local",
        ]
        .into_iter()
        .map(|p| protect(p, Itself)),
    );
    rules.push(Rule::Except(PathBuf::from("/usr/local")));
    rules
}

#[cfg(windows)]
fn os_rules() -> Vec<Rule> {
    use Reach::{Itself, Subtree};
    let drive = std::env::var_os("SystemDrive").map_or_else(
        || PathBuf::from("C:\\"),
        |mut d| {
            // `C:` alone means "the current folder on C:"; the root needs the separator.
            d.push("\\");
            PathBuf::from(d)
        },
    );
    let var = |name: &str, fallback: &str| {
        std::env::var_os(name).map_or_else(|| drive.join(fallback), PathBuf::from)
    };
    vec![
        Rule::Protect(var("SystemRoot", "Windows"), Subtree),
        Rule::Protect(var("ProgramFiles", "Program Files"), Subtree),
        Rule::Protect(var("ProgramFiles(x86)", "Program Files (x86)"), Subtree),
        Rule::Protect(var("ProgramData", "ProgramData"), Itself),
        Rule::Protect(drive.join("Users"), Itself),
    ]
}

#[cfg(test)]
mod tests {
    use super::{Protected, Reach, Rule, system_rules};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cleany-protected-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mkdirs(dir: &Path, rels: &[&str]) {
        for r in rels {
            fs::create_dir_all(dir.join(r)).unwrap();
        }
    }

    /// `sys/usr` (subtree) with `sys/usr/local` excepted and kept itself,
    /// `sys/Library` kept itself.
    fn fake(dir: &Path) -> Protected {
        mkdirs(
            dir,
            &[
                "sys/usr/bin",
                "sys/usr/local/Cellar",
                "sys/Library/Caches",
                "sys/other",
            ],
        );
        Protected::resolve(&[
            Rule::Protect(dir.join("sys/usr"), Reach::Subtree),
            Rule::Protect(dir.join("sys/usr/local"), Reach::Itself),
            Rule::Except(dir.join("sys/usr/local")),
            Rule::Protect(dir.join("sys/Library"), Reach::Itself),
            Rule::Protect(dir.join("sys/missing"), Reach::Subtree),
        ])
    }

    #[test]
    fn itself_subtree_exception_and_holders() {
        let dir = scratch("reach");
        let p = fake(&dir);
        // Itself: the folder, not its contents.
        assert!(p.covers(&dir.join("sys/Library")));
        assert!(!p.covers(&dir.join("sys/Library/Caches")));
        // Subtree: the folder and everything below.
        assert!(p.covers(&dir.join("sys/usr")));
        assert!(p.covers(&dir.join("sys/usr/bin")));
        // The exception: kept itself, contents free.
        assert!(p.covers(&dir.join("sys/usr/local")));
        assert!(!p.covers(&dir.join("sys/usr/local/Cellar")));
        // A folder holding a protected entry.
        assert!(p.covers(&dir.join("sys")));
        assert!(p.covers(&dir));
        // Unrelated, and missing paths.
        assert!(!p.covers(&dir.join("sys/other")));
        assert!(!p.covers(&dir.join("sys/missing")));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_are_judged_as_links() {
        use std::os::unix::fs::symlink;
        let dir = scratch("links");
        mkdirs(&dir, &["sys/usr/bin", "home"]);
        // A listed folder that is itself a link, as `/bin` on merged-usr.
        symlink(dir.join("sys/usr/bin"), dir.join("sys/bin")).unwrap();
        // A user's link into a protected folder.
        symlink(dir.join("sys/usr"), dir.join("home/usr-link")).unwrap();
        let p = Protected::resolve(&[
            Rule::Protect(dir.join("sys/usr"), Reach::Subtree),
            Rule::Protect(dir.join("sys/bin"), Reach::Subtree),
        ]);
        assert!(p.covers(&dir.join("sys/bin")));
        // Reached through a link, the real folder is still under `usr`.
        assert!(p.covers(&dir.join("sys/bin/.")));
        assert!(p.covers(&dir.join("home/usr-link/bin")));
        // The link itself is just a link.
        assert!(!p.covers(&dir.join("home/usr-link")));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn other_letter_case_still_matches() {
        let dir = scratch("case");
        let p = fake(&dir);
        // APFS is case-insensitive by default; skip on a case-sensitive one.
        if dir.join("SYS").exists() {
            assert!(p.covers(&dir.join("SYS/USR/bin")));
            assert!(p.covers(&dir.join("sys/library")));
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn real_macos_list() {
        let home = std::env::home_dir();
        let p = Protected::resolve(&system_rules(home.as_deref()));
        for path in [
            "/System/Library",
            "/usr/bin",
            "/usr/local",
            "/Applications",
            "/etc",
            "/etc/hosts",
            "/private",
            "/Library",
            "/Users",
        ] {
            assert!(p.covers(Path::new(path)), "{path}");
        }
        assert!(!p.covers(Path::new("/Library/Caches")));
        if let Some(home) = home {
            assert!(p.covers(&home.join("Library")));
            assert!(!p.covers(&home.join("Library/Caches")));
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn real_linux_list() {
        let p = Protected::resolve(&system_rules(None));
        for path in ["/usr/bin", "/etc", "/etc/passwd", "/usr"] {
            assert!(p.covers(Path::new(path)), "{path}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn real_windows_list() {
        let p = Protected::resolve(&system_rules(None));
        let root = std::env::var_os("SystemRoot")
            .map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from);
        assert!(p.covers(&root));
        assert!(p.covers(&root.join("System32")));
    }
}
