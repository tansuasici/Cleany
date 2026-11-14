//! Mounted volume listing for the start screen.

use std::path::{Path, PathBuf};

use sysinfo::{Disk, Disks};

use crate::Os;

/// One mounted volume the user can pick as a scan root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Where the volume is mounted (`/`, `/Volumes/USB`, `C:\`).
    pub mount: PathBuf,
    /// Human-readable label; falls back to the mount point when the OS has none.
    pub name: String,
    /// Capacity in bytes.
    pub total: u64,
    /// Bytes available to the current user.
    pub available: u64,
    /// Whether the OS reports the volume as removable media.
    pub removable: bool,
}

/// Lists the mounted volumes, sorted by mount point, without duplicates.
///
/// Volumes with zero capacity (pseudo filesystems) are skipped, and so are
/// macOS APFS system mounts under `/System/Volumes/` (Finder hides them too;
/// the user data volume is reachable from `/` through firmlinks). Never
/// fails: an OS that reports nothing yields an empty list.
#[must_use]
pub fn list_volumes() -> Vec<Volume> {
    let disks = Disks::new_with_refreshed_list();
    collect(disks.list().iter().map(volume_from_disk), Os::current())
}

/// Whether a mount is an OS-internal volume the user should not pick.
fn is_hidden(mount: &Path, os: Os) -> bool {
    os == Os::Macos && mount.starts_with("/System/Volumes")
}

fn volume_from_disk(disk: &Disk) -> Volume {
    let mount = disk.mount_point().to_path_buf();
    let label = disk.name().to_string_lossy();
    let name = if label.trim().is_empty() {
        mount.display().to_string()
    } else {
        label.into_owned()
    };
    Volume {
        mount,
        name,
        total: disk.total_space(),
        available: disk.available_space(),
        removable: disk.is_removable(),
    }
}

/// Drops pseudo filesystems and hidden system mounts, sorts by mount point
/// and removes repeated mounts.
fn collect(volumes: impl Iterator<Item = Volume>, os: Os) -> Vec<Volume> {
    let mut out: Vec<Volume> = volumes
        .filter(|v| v.total > 0 && !is_hidden(&v.mount, os))
        .collect();
    out.sort_by(|a, b| a.mount.cmp(&b.mount));
    out.dedup_by(|a, b| a.mount == b.mount);
    out
}

#[cfg(test)]
mod tests {
    use super::{Volume, collect, list_volumes};
    use crate::Os;
    use std::path::PathBuf;

    fn vol(mount: &str, total: u64) -> Volume {
        Volume {
            mount: PathBuf::from(mount),
            name: mount.to_owned(),
            total,
            available: total / 2,
            removable: false,
        }
    }

    #[test]
    fn collect_sorts_dedups_and_drops_empty() {
        let got = collect(
            [vol("/b", 10), vol("/a", 10), vol("/proc", 0), vol("/b", 10)].into_iter(),
            Os::Linux,
        );
        let mounts: Vec<_> = got.iter().map(|v| v.mount.clone()).collect();
        assert_eq!(mounts, [PathBuf::from("/a"), PathBuf::from("/b")]);
    }

    #[test]
    fn macos_system_volumes_are_hidden_only_on_macos() {
        let input = || {
            [
                vol("/", 10),
                vol("/System/Volumes/Data", 10),
                vol("/Volumes/USB", 10),
            ]
            .into_iter()
        };
        let mounts =
            |os| -> Vec<PathBuf> { collect(input(), os).into_iter().map(|v| v.mount).collect() };
        assert_eq!(
            mounts(Os::Macos),
            [PathBuf::from("/"), PathBuf::from("/Volumes/USB")]
        );
        assert_eq!(mounts(Os::Linux).len(), 3);
    }

    #[test]
    fn real_volumes_are_consistent() {
        let volumes = list_volumes();
        for v in &volumes {
            assert!(v.total > 0, "{v:?}");
            assert!(v.available <= v.total, "{v:?}");
            assert!(!v.name.is_empty(), "{v:?}");
            assert!(v.mount.is_absolute(), "{v:?}");
        }
        // CI runners and dev machines always have a root/system volume.
        assert!(!volumes.is_empty());
    }
}
