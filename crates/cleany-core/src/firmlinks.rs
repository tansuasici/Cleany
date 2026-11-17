//! macOS firmlinks: folders of the Data volume that also appear at the root.
//!
//! Since macOS 10.15 the system is split into a read-only System volume and
//! a Data volume mounted at `/System/Volumes/Data`. Each line of
//! `/usr/share/firmlinks` joins one Data folder to a path on the System
//! volume (`/Users`, `/Applications`, `/private`, `/usr/local`, …), so the
//! same folder is reachable twice: at its source (`/Users`) and at its
//! alias (`/System/Volumes/Data/Users`). Both report the same device and
//! file id, so neither the mount check nor hard-link grouping notices.
//!
//! A scan whose original root holds both sides enters only the source; the
//! alias is recorded but not entered. A scan rooted inside the Data volume
//! does not hold the sources, so nothing is skipped there. Every other OS
//! has an empty table.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use cleany_platform::{EntryKind, read_meta};

/// Where the Data volume is mounted.
const DATA_ROOT: &str = "/System/Volumes/Data";
/// The system's firmlink list: `source<TAB>target relative to DATA_ROOT`.
const TABLE_PATH: &str = "/usr/share/firmlinks";

/// One folder reachable from two paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Firmlink {
    /// The path on the System volume side (`/Users`).
    pub(crate) source: PathBuf,
    /// The same folder below [`DATA_ROOT`] (`/System/Volumes/Data/Users`).
    pub(crate) alias: PathBuf,
}

/// Parses the text of `/usr/share/firmlinks`. Malformed lines are dropped.
pub(crate) fn parse(text: &str) -> Vec<Firmlink> {
    text.lines()
        .filter_map(|line| {
            let (source, target) = line.split_once('\t')?;
            let (source, target) = (source.trim(), target.trim());
            if !source.starts_with('/') || target.is_empty() || target.starts_with('/') {
                return None;
            }
            Some(Firmlink {
                source: PathBuf::from(source),
                alias: Path::new(DATA_ROOT).join(target),
            })
        })
        .collect()
}

/// Whether both sides of `link` exist and are the very same directory.
/// A stale or absent row is ignored rather than hiding real data.
fn is_live(link: &Firmlink) -> bool {
    match (read_meta(&link.source), read_meta(&link.alias)) {
        (Ok(s), Ok(a)) => {
            s.kind == EntryKind::Dir
                && a.kind == EntryKind::Dir
                && s.file_id != 0
                && (s.dev, s.file_id) == (a.dev, a.file_id)
        }
        _ => false,
    }
}

/// The firmlinks of this system, read and checked once per process.
fn system_table() -> &'static [Firmlink] {
    static TABLE: OnceLock<Vec<Firmlink>> = OnceLock::new();
    TABLE.get_or_init(|| {
        if !cfg!(target_os = "macos") {
            return Vec::new();
        }
        fs::read_to_string(TABLE_PATH)
            .map(|text| parse(&text).into_iter().filter(is_live).collect())
            .unwrap_or_default()
    })
}

#[cfg(test)]
thread_local! {
    /// Replaces [`system_table`] on this thread, for tests.
    static TEST_TABLE: std::cell::RefCell<Option<Vec<Firmlink>>> =
        const { std::cell::RefCell::new(None) };
}

/// Makes [`aliases_below`] use `links` on this thread until dropped.
#[cfg(test)]
pub(crate) fn override_table(links: Vec<Firmlink>) -> impl Drop {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_TABLE.with(|t| *t.borrow_mut() = None);
        }
    }
    TEST_TABLE.with(|t| *t.borrow_mut() = Some(links));
    Reset
}

/// Aliases of `links` strictly below `walk` whose source lies under
/// `origin`. Both paths are compared as given.
fn select<'a>(
    links: &'a [Firmlink],
    origin: &'a Path,
    walk: &'a Path,
) -> impl Iterator<Item = &'a Path> {
    links
        .iter()
        .filter(move |l| l.source.starts_with(origin) && l.alias.starts_with(walk))
        .map(|l| l.alias.as_path())
        .filter(move |a| *a != walk)
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Directories not to enter while walking `walk_root` for a scan whose
/// original root is `origin`, spelled under `walk_root` so they can be
/// compared with the paths the walk builds.
pub(crate) fn aliases_below(origin: &Path, walk_root: &Path) -> Vec<PathBuf> {
    let pick = |links: &[Firmlink]| -> Vec<PathBuf> {
        if links.is_empty() {
            return Vec::new();
        }
        let (origin_c, walk_c) = (canonical(origin), canonical(walk_root));
        select(links, &origin_c, &walk_c)
            .filter_map(|a| a.strip_prefix(&walk_c).ok())
            .map(|rest| walk_root.join(rest))
            .collect()
    };
    #[cfg(test)]
    if let Some(found) = TEST_TABLE.with(|t| t.borrow().as_deref().map(pick)) {
        return found;
    }
    pick(system_table())
}

/// Whether `path` is an alias a scan rooted at `origin` does not enter.
pub(crate) fn is_alias(origin: &Path, path: &Path) -> bool {
    path.parent()
        .is_some_and(|parent| aliases_below(origin, parent).iter().any(|a| a == path))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Firmlink, parse, select};

    const SAMPLE: &str = "/Applications\tApplications\n\
        /System/Library/Caches\tSystem/Library/Caches\n\
        /Users\tUsers\n\
        /usr/local\tusr/local\n\
        not a firmlink line\n\
        /bad\t/absolute\n";

    fn links() -> Vec<Firmlink> {
        parse(SAMPLE)
    }

    fn picked(origin: &str, walk: &str) -> Vec<PathBuf> {
        select(&links(), Path::new(origin), Path::new(walk))
            .map(Path::to_path_buf)
            .collect()
    }

    #[test]
    fn parse_maps_targets_under_the_data_volume() {
        let got = links();
        assert_eq!(got.len(), 4);
        assert_eq!(
            got[2],
            Firmlink {
                source: PathBuf::from("/Users"),
                alias: PathBuf::from("/System/Volumes/Data/Users"),
            }
        );
    }

    #[test]
    fn root_scan_skips_every_alias() {
        assert_eq!(picked("/", "/").len(), 4);
        // A subtree rescan of the Data volume still belongs to the `/` scan.
        assert_eq!(
            picked("/", "/System/Volumes/Data/usr"),
            [PathBuf::from("/System/Volumes/Data/usr/local")]
        );
    }

    #[test]
    fn system_scan_skips_only_aliases_of_its_own_sources() {
        assert_eq!(
            picked("/System", "/System"),
            [PathBuf::from("/System/Volumes/Data/System/Library/Caches")]
        );
    }

    #[test]
    fn scans_without_both_sides_skip_nothing() {
        // Rooted in the Data volume: the sources are outside the scan.
        assert!(picked("/System/Volumes/Data", "/System/Volumes/Data").is_empty());
        // Rooted at a source: the aliases are outside the scan.
        assert!(picked("/Users", "/Users").is_empty());
        // The alias itself as the walk root is never skipped.
        assert!(picked("/", "/System/Volumes/Data/Users").is_empty());
    }
}
