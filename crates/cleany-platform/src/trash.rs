//! Moving items to the OS trash, and the paths Cleany never trashes.
//!
//! The path to trash is never canonicalized: a symlink is trashed as the link,
//! not its target. Only the protection checks resolve paths, to compare them.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use sysinfo::Disks;

use crate::meta::read_meta;
use crate::protected::{Protected, system_rules};

/// Why an item could not be moved to the trash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrashError {
    /// The item no longer exists.
    Gone,
    /// The OS refused or failed; the text is for the log.
    Failed(String),
}

impl fmt::Display for TrashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gone => f.write_str("the item no longer exists"),
            Self::Failed(message) => write!(f, "cannot move to trash: {message}"),
        }
    }
}

impl std::error::Error for TrashError {}

/// Moves `path` to the OS trash (Finder Trash, Recycle Bin, freedesktop
/// trash). On macOS this asks `NSFileManager` directly, so no automation
/// prompt appears; "Put Back" is then not offered for the item.
///
/// # Errors
/// [`TrashError::Gone`] when nothing is at `path`, [`TrashError::Failed`]
/// when the OS does not move it.
pub fn move_to_trash(path: &Path) -> Result<(), TrashError> {
    if let Err(e) = std::fs::symlink_metadata(path) {
        return Err(if e.kind() == io::ErrorKind::NotFound {
            TrashError::Gone
        } else {
            TrashError::Failed(e.to_string())
        });
    }
    context().delete(path).map_err(|e| {
        // Removed by someone else between the check and the move.
        if std::fs::symlink_metadata(path).is_err() {
            TrashError::Gone
        } else {
            TrashError::Failed(e.to_string())
        }
    })
}

/// The trash context; macOS uses `NSFileManager` instead of scripting Finder.
#[cfg(target_os = "macos")]
fn context() -> trash::TrashContext {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    let mut ctx = trash::TrashContext::new();
    ctx.set_delete_method(DeleteMethod::NsFileManager);
    ctx
}

/// The trash context, with the OS defaults.
#[cfg(not(target_os = "macos"))]
fn context() -> trash::TrashContext {
    trash::TrashContext::new()
}

/// What Cleany refuses to trash, gathered once per request: the home folder,
/// the mount points and the OS folders listed for this platform.
#[derive(Debug)]
pub struct Protection {
    home: Option<PathBuf>,
    mounts: Vec<PathBuf>,
    system: Protected,
}

impl Protection {
    /// Reads the home folder, the mounted disks and the OS folder list.
    #[must_use]
    pub fn current() -> Self {
        let home = std::env::home_dir();
        let mounts = Disks::new_with_refreshed_list()
            .list()
            .iter()
            .map(|d| d.mount_point().to_path_buf())
            .collect();
        let system = Protected::resolve(&system_rules(home.as_deref()));
        Self {
            home,
            mounts,
            system,
        }
    }

    /// Whether Cleany refuses to trash `path`: the scan root, a filesystem
    /// root, the home folder or any folder above it, a folder that is or
    /// contains a mount point, or an OS folder (`/System`, `/usr`,
    /// `C:\Windows`, `~/Library`…; some only as the folder itself, see the
    /// list in `protected.rs`). Paths are compared by identity, so `/var` and
    /// `/private/var` or a symlinked home are the same folder.
    #[must_use]
    pub fn covers(&self, path: &Path, scan_root: &Path) -> bool {
        is_protected_with(
            path,
            scan_root,
            self.home.as_deref(),
            &self.mounts,
            &self.system,
        )
    }
}

/// [`Protection::covers`] for a single path; build a [`Protection`] once for
/// several.
#[must_use]
pub fn is_protected(path: &Path, scan_root: &Path) -> bool {
    Protection::current().covers(path, scan_root)
}

/// [`Protection::covers`] with every input given, for tests.
fn is_protected_with(
    path: &Path,
    scan_root: &Path,
    home: Option<&Path>,
    mounts: &[PathBuf],
    system: &Protected,
) -> bool {
    if path.parent().is_none() || same_entry(path, scan_root) {
        return true;
    }
    if let Some(home) = home
        && home.ancestors().any(|a| same_entry(path, a))
    {
        return true;
    }
    // Before the symlink rule: a listed folder that is a link stays.
    if system.covers(path) {
        return true;
    }
    // A symlink is trashed as the link; what it points at stays.
    let is_link = std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink());
    if is_link {
        return false;
    }
    let Ok(real) = path.canonicalize() else {
        return false;
    };
    if real.parent().is_none() {
        return true;
    }
    mounts.iter().any(|m| {
        let m = m.canonicalize().unwrap_or_else(|_| m.clone());
        m.starts_with(&real)
    })
}

/// Whether `a` and `b` name the same filesystem entry (symlinks at the last
/// component are not followed). Falls back to comparing canonical paths when
/// the OS gives no file ids.
fn same_entry(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (read_meta(a), read_meta(b)) {
        (Ok(x), Ok(y)) if x.file_id != 0 && y.file_id != 0 => {
            x.dev == y.dev && x.file_id == y.file_id
        }
        (Ok(_), Ok(_)) => match (a.canonicalize(), b.canonicalize()) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{TrashError, is_protected_with, move_to_trash};
    use crate::protected::Protected;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// [`is_protected_with`] with no OS folder list.
    fn check(path: &Path, root: &Path, home: Option<&Path>, mounts: &[PathBuf]) -> bool {
        is_protected_with(path, root, home, mounts, &Protected::default())
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cleany-trash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roots_scan_root_and_home_chain_are_protected() {
        let dir = scratch("protect");
        let home = dir.join("home/me");
        let other = dir.join("other");
        fs::create_dir_all(home.join("Downloads")).unwrap();
        fs::create_dir_all(&other).unwrap();
        let none: &[PathBuf] = &[];

        assert!(check(Path::new("/"), &other, Some(&home), none));
        assert!(check(&other, &other, Some(&home), none));
        assert!(check(&home, &other, Some(&home), none));
        assert!(check(&dir.join("home"), &other, Some(&home), none));
        assert!(check(&dir, &other, Some(&home), none));
        assert!(!check(&home.join("Downloads"), &other, Some(&home), none));
        // A trailing `.` is the same folder.
        assert!(check(&home.join("."), &other, Some(&home), none));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn folders_holding_a_mount_are_protected() {
        let dir = scratch("mounts");
        let vol = dir.join("Volumes/USB");
        fs::create_dir_all(&vol).unwrap();
        fs::create_dir_all(dir.join("plain")).unwrap();
        let mounts = vec![vol.clone()];
        let root = Path::new("/nonexistent-root");

        assert!(check(&vol, root, None, &mounts));
        assert!(check(&dir.join("Volumes"), root, None, &mounts));
        assert!(!check(&dir.join("plain"), root, None, &mounts));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_spellings_match_and_links_are_not_followed() {
        use super::same_entry;

        let dir = scratch("links");
        let real = dir.join("real");
        fs::create_dir_all(real.join("inner")).unwrap();
        let link = dir.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        // Through the link, `inner` is the same folder.
        assert!(same_entry(&link.join("inner"), &real.join("inner")));
        // The link itself is not its target.
        assert!(!same_entry(&link, &real));
        // Home reached through a link still protects the real folder.
        assert!(check(&real, Path::new("/x"), Some(&link.join(".")), &[]));
        // A link pointing at a mount holder is just a link.
        let mounts = vec![real.join("inner")];
        assert!(!check(&link, Path::new("/x"), None, &mounts));
        assert!(check(&real, Path::new("/x"), None, &mounts));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn listed_links_stay_and_the_list_applies() {
        use crate::protected::{Reach, Rule};

        let dir = scratch("listed");
        fs::create_dir_all(dir.join("usr/bin")).unwrap();
        fs::create_dir_all(dir.join("free")).unwrap();
        std::os::unix::fs::symlink(dir.join("usr/bin"), dir.join("bin")).unwrap();
        let system = Protected::resolve(&[
            Rule::Protect(dir.join("usr"), Reach::Subtree),
            Rule::Protect(dir.join("bin"), Reach::Subtree),
        ]);
        let root = Path::new("/x");
        // A listed link is caught before the "links are just links" rule.
        assert!(is_protected_with(
            &dir.join("bin"),
            root,
            None,
            &[],
            &system
        ));
        assert!(is_protected_with(
            &dir.join("usr/bin"),
            root,
            None,
            &[],
            &system
        ));
        assert!(!is_protected_with(
            &dir.join("free"),
            root,
            None,
            &[],
            &system
        ));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_item_is_gone_without_touching_the_trash() {
        let dir = scratch("gone");
        assert_eq!(move_to_trash(&dir.join("nope")), Err(TrashError::Gone));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Touches the real OS trash; run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "moves a scratch file into the real OS trash"]
    fn moves_a_scratch_file_to_the_real_trash() {
        let dir = scratch("real");
        let file = dir.join("cleany-trash-test.txt");
        fs::write(&file, b"cleany").unwrap();
        move_to_trash(&file).unwrap();
        assert!(!file.exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
