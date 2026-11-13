//! Raw per-entry metadata, read without following links (ADR-0005).
//!
//! M1 uses `std::fs` only. On Windows, `dev`, `file_id` and `nlink` are not
//! available from stable `std` and are reported as `0, 0, 1` until the
//! Windows helpers land (M1 PR 4).

use std::fs::{self, FileType, Metadata};
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// What an entry is, as seen without following links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symbolic link, junction or other name-surrogate reparse point.
    Symlink,
    /// Socket, FIFO, device or anything else.
    Other,
}

impl EntryKind {
    /// Classifies a [`FileType`] obtained without following links.
    #[must_use]
    pub fn from_file_type(ft: FileType) -> Self {
        if ft.is_symlink() {
            Self::Symlink
        } else if ft.is_dir() {
            Self::Dir
        } else if ft.is_file() {
            Self::File
        } else {
            Self::Other
        }
    }
}

/// Raw metadata for one entry. Sizes are bytes; `mtime` is seconds since the
/// Unix epoch (negative before it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntryMeta {
    /// Entry kind.
    pub kind: EntryKind,
    /// Logical length in bytes.
    pub logical: u64,
    /// Bytes allocated on disk.
    pub allocated: u64,
    /// Device / volume id (`0` where unknown).
    pub dev: u64,
    /// Inode / file id on that device (`0` where unknown).
    pub file_id: u64,
    /// Hard link count (`1` where unknown).
    pub nlink: u64,
    /// Modification time in seconds since the Unix epoch.
    pub mtime: i64,
}

impl EntryMeta {
    /// Converts metadata that was read without following links.
    #[must_use]
    pub fn from_metadata(md: &Metadata) -> Self {
        let kind = EntryKind::from_file_type(md.file_type());
        let ids = raw_ids(md);
        // Windows: a directory's NTFS index size is reported by
        // `symlink_metadata` but not by `DirEntry::metadata` (find data), so
        // directories count as 0 there to keep both sources consistent.
        let (logical, allocated) = if cfg!(windows) && kind == EntryKind::Dir {
            (0, 0)
        } else {
            (md.len(), ids.allocated)
        };
        Self {
            kind,
            logical,
            allocated,
            dev: ids.dev,
            file_id: ids.file_id,
            nlink: ids.nlink,
            mtime: mtime_secs(md),
        }
    }
}

/// Reads metadata for `path` without following a final symlink.
///
/// # Errors
/// Any I/O error from `symlink_metadata`.
pub fn read_meta(path: &Path) -> io::Result<EntryMeta> {
    fs::symlink_metadata(path).map(|md| EntryMeta::from_metadata(&md))
}

struct RawIds {
    allocated: u64,
    dev: u64,
    file_id: u64,
    nlink: u64,
}

#[cfg(unix)]
fn raw_ids(md: &Metadata) -> RawIds {
    use std::os::unix::fs::MetadataExt;
    RawIds {
        // `st_blocks` is always in 512-byte units, whatever the FS block size.
        allocated: md.blocks().saturating_mul(512),
        dev: md.dev(),
        file_id: md.ino(),
        nlink: md.nlink(),
    }
}

#[cfg(windows)]
fn raw_ids(md: &Metadata) -> RawIds {
    // Stable std has no allocation size, volume serial or link count;
    // PR 4 replaces this with the real values.
    RawIds {
        allocated: md.len(),
        dev: 0,
        file_id: 0,
        nlink: 1,
    }
}

fn mtime_secs(md: &Metadata) -> i64 {
    let Ok(t) = md.modified() else { return 0 };
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => i64::try_from(e.duration().as_secs()).map_or(i64::MIN, |s| -s),
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryKind, read_meta};
    use std::fs;

    #[test]
    fn read_meta_reports_file_and_dir() {
        let dir = std::env::temp_dir().join(format!("cleany-meta-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("f.bin");
        fs::write(&file, vec![0u8; 10_000]).unwrap();

        let d = read_meta(&dir).unwrap();
        let f = read_meta(&file).unwrap();
        assert_eq!(d.kind, EntryKind::Dir);
        assert_eq!(f.kind, EntryKind::File);
        assert_eq!(f.logical, 10_000);
        assert!(f.allocated > 0);
        assert!(f.mtime > 0);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn read_meta_does_not_follow_symlinks() {
        let dir = std::env::temp_dir().join(format!("cleany-meta-link-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let link = dir.join("l");
        std::os::unix::fs::symlink("/", &link).unwrap();
        assert_eq!(read_meta(&link).unwrap().kind, EntryKind::Symlink);
        fs::remove_dir_all(&dir).unwrap();
    }
}
