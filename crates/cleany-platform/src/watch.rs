//! Live change notifications below a scanned root.
//!
//! On macOS this is FSEvents through the `notify` crate; every other target
//! gets [`WatchError::Unsupported`] and keeps today's behaviour (the scan is a
//! snapshot). Events are reported as plain paths under the root the caller
//! gave, in the caller's spelling: FSEvents reports resolved paths
//! (`/private/var/...` for `/var/...`), which are mapped back here so they
//! match the scanned tree. Nothing is debounced here; merging belongs to
//! `cleany_core::refresh::Coalescer`.
//!
//! Known limits:
//! - `notify` 8 panics inside its FSEvents callback on a path that is not
//!   valid UTF-8 or on an event flag it does not know. It knows every flag
//!   in the macOS 27.2 SDK and APFS/HFS+ names are UTF-8, but network
//!   volumes may not be; with `panic = "abort"` such a panic ends the app.
//! - Without Full Disk Access, macOS may not report changes inside protected
//!   folders; those parts of the tree stay as scanned.

use std::fmt;
use std::path::{Path, PathBuf};

/// How much may have changed at a reported path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// The entry itself was created, removed, renamed or written.
    Entry,
    /// Anything below the path may have changed (events were dropped or
    /// merged by the OS); the whole subtree needs a rescan.
    Subtree,
}

/// One change below the watched root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsChange {
    /// The changed path, spelled under the root given to [`watch`].
    pub path: PathBuf,
    /// How much may have changed there.
    pub kind: ChangeKind,
}

/// Why watching could not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchError {
    /// This OS has no live watching yet.
    Unsupported,
    /// The OS refused or failed; the text is for the log.
    Failed(String),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("live watching is not supported on this OS"),
            Self::Failed(message) => write!(f, "cannot watch for changes: {message}"),
        }
    }
}

impl std::error::Error for WatchError {}

/// A running watch. Dropping it stops the OS stream; no callback runs after
/// the drop returns.
pub struct Watcher {
    #[cfg(target_os = "macos")]
    _inner: notify::FsEventWatcher,
}

impl fmt::Debug for Watcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Watcher").finish_non_exhaustive()
    }
}

/// Starts watching everything below `root` (recursively) and calls
/// `on_change` from a background thread for every change.
///
/// `on_change` must not panic and should return quickly (push into a channel
/// or a buffer); events are delivered one at a time.
///
/// # Errors
/// [`WatchError::Unsupported`] outside macOS, [`WatchError::Failed`] when the
/// root cannot be resolved or the OS stream does not start.
pub fn watch<F>(root: &Path, on_change: F) -> Result<Watcher, WatchError>
where
    F: FnMut(FsChange) + Send + 'static,
{
    #[cfg(target_os = "macos")]
    {
        mac::start(root, on_change).map(|inner| Watcher { _inner: inner })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (root, on_change);
        Err(WatchError::Unsupported)
    }
}

/// Maps paths the OS reports back under the root the caller asked for.
#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone)]
struct Rebase {
    /// The root as the caller spelled it (the scanned tree's root).
    root: PathBuf,
    /// The same root with symlinks resolved, as FSEvents reports it.
    real: PathBuf,
}

#[cfg(any(target_os = "macos", test))]
impl Rebase {
    /// `path` under [`Self::root`], or `None` when it lies outside the root.
    fn map(&self, path: &Path) -> Option<PathBuf> {
        let rest = path
            .strip_prefix(&self.real)
            .or_else(|_| path.strip_prefix(&self.root))
            .ok()?;
        if rest.as_os_str().is_empty() {
            Some(self.root.clone())
        } else {
            Some(self.root.join(rest))
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::path::Path;

    use notify::event::{EventKind, MetadataKind, ModifyKind};
    use notify::{Config, Event, FsEventWatcher, RecursiveMode, Watcher as _};

    use super::{ChangeKind, FsChange, Rebase, WatchError};

    fn failed(err: impl std::fmt::Display) -> WatchError {
        WatchError::Failed(err.to_string())
    }

    pub(super) fn start<F>(root: &Path, mut on_change: F) -> Result<FsEventWatcher, WatchError>
    where
        F: FnMut(FsChange) + Send + 'static,
    {
        let real = root.canonicalize().map_err(failed)?;
        let rebase = Rebase {
            root: root.to_path_buf(),
            real,
        };
        let handler = move |event: notify::Result<Event>| {
            for change in translate(event, &rebase) {
                on_change(change);
            }
        };
        let mut watcher = FsEventWatcher::new(handler, Config::default()).map_err(failed)?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(failed)?;
        Ok(watcher)
    }

    /// Turns one `notify` result into changes under the caller's root.
    pub(super) fn translate(event: notify::Result<Event>, rebase: &Rebase) -> Vec<FsChange> {
        let (paths, kind) = match event {
            // An error with no path means the stream itself is unreliable now.
            Err(err) if err.paths.is_empty() => {
                return vec![FsChange {
                    path: rebase.root.clone(),
                    kind: ChangeKind::Subtree,
                }];
            }
            Err(err) => (err.paths, ChangeKind::Subtree),
            Ok(event) if event.need_rescan() => (event.paths, ChangeKind::Subtree),
            Ok(event) if is_noise(&event.kind) => return Vec::new(),
            Ok(event) => (event.paths, ChangeKind::Entry),
        };
        paths
            .iter()
            .filter_map(|path| rebase.map(path))
            .map(|path| FsChange { path, kind })
            .collect()
    }

    /// Changes that never move a size, a kind or a name: reads, owner,
    /// permission, extended-attribute and Finder-info updates.
    fn is_noise(kind: &EventKind) -> bool {
        matches!(
            kind,
            EventKind::Access(_)
                | EventKind::Modify(ModifyKind::Metadata(
                    MetadataKind::Ownership
                        | MetadataKind::Permissions
                        | MetadataKind::Extended
                        | MetadataKind::Other
                ))
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::Rebase;

    fn rebase() -> Rebase {
        Rebase {
            root: PathBuf::from("/var/scan"),
            real: PathBuf::from("/private/var/scan"),
        }
    }

    #[test]
    fn rebase_maps_resolved_paths_back_to_the_callers_root() {
        let r = rebase();
        assert_eq!(
            r.map(Path::new("/private/var/scan/a/b.txt")),
            Some(PathBuf::from("/var/scan/a/b.txt"))
        );
        assert_eq!(
            r.map(Path::new("/private/var/scan")),
            Some(PathBuf::from("/var/scan"))
        );
        // Already in the caller's spelling.
        assert_eq!(
            r.map(Path::new("/var/scan/c")),
            Some(PathBuf::from("/var/scan/c"))
        );
    }

    #[test]
    fn rebase_drops_paths_outside_the_root() {
        let r = rebase();
        assert_eq!(r.map(Path::new("/private/var/other/x")), None);
        // A sibling that only shares a string prefix.
        assert_eq!(r.map(Path::new("/private/var/scanner/x")), None);
    }

    #[test]
    fn watcher_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<super::Watcher>();
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn watch_is_unsupported_off_macos() {
        let err = super::watch(Path::new("."), |_| {}).unwrap_err();
        assert_eq!(err, super::WatchError::Unsupported);
    }

    #[cfg(target_os = "macos")]
    mod mac {
        use std::path::{Path, PathBuf};

        use notify::Event;
        use notify::event::Flag;
        use notify::event::{
            CreateKind, DataChange, EventKind, MetadataKind, ModifyKind, RemoveKind,
        };

        use super::rebase;
        use crate::watch::mac::translate;
        use crate::watch::{ChangeKind, FsChange};

        fn event(kind: EventKind, path: &str) -> Event {
            Event::new(kind).add_path(PathBuf::from(path))
        }

        fn change(path: &str, kind: ChangeKind) -> FsChange {
            FsChange {
                path: PathBuf::from(path),
                kind,
            }
        }

        #[test]
        fn translate_reports_entry_changes_under_the_callers_root() {
            let r = rebase();
            for kind in [
                EventKind::Create(CreateKind::File),
                EventKind::Remove(RemoveKind::Folder),
                EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)),
            ] {
                assert_eq!(
                    translate(Ok(event(kind, "/private/var/scan/a")), &r),
                    vec![change("/var/scan/a", ChangeKind::Entry)],
                    "{kind:?}"
                );
            }
        }

        #[test]
        fn translate_turns_rescan_and_errors_into_subtree_changes() {
            let r = rebase();
            let rescan = event(EventKind::Other, "/private/var/scan/d").set_flag(Flag::Rescan);
            assert_eq!(
                translate(Ok(rescan), &r),
                vec![change("/var/scan/d", ChangeKind::Subtree)]
            );
            let with_path =
                notify::Error::generic("x").add_path(PathBuf::from("/private/var/scan/e"));
            assert_eq!(
                translate(Err(with_path), &r),
                vec![change("/var/scan/e", ChangeKind::Subtree)]
            );
            assert_eq!(
                translate(Err(notify::Error::generic("x")), &r),
                vec![change("/var/scan", ChangeKind::Subtree)]
            );
        }

        #[test]
        fn translate_skips_noise_and_paths_outside_the_root() {
            let r = rebase();
            let xattr = EventKind::Modify(ModifyKind::Metadata(MetadataKind::Extended));
            assert!(translate(Ok(event(xattr, "/private/var/scan/a")), &r).is_empty());
            let outside = EventKind::Create(CreateKind::File);
            assert!(translate(Ok(event(outside, "/elsewhere/a")), &r).is_empty());
        }

        #[test]
        fn translate_keeps_non_ascii_components_verbatim() {
            let r = rebase();
            let kind = EventKind::Create(CreateKind::File);
            assert_eq!(
                translate(Ok(event(kind, "/private/var/scan/Ünï")), &r),
                vec![FsChange {
                    path: Path::new("/var/scan").join("Ünï"),
                    kind: ChangeKind::Entry,
                }]
            );
        }
    }
}
