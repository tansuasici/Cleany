//! Raw per-entry metadata, read without following links (ADR-0005).
//!
//! Unix values come straight from `stat`. On Windows the find data is
//! completed with a cheap attributes-only handle (volume serial, file index,
//! link count) and `GetCompressedFileSizeW` (allocated size). Cloud
//! placeholders are classified from flags alone; their contents are never
//! opened, so scanning them does not trigger a download.

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
    /// A cloud placeholder (iCloud / FileProvider dataless object, OneDrive
    /// or other cloud-files placeholder) whose data is not stored locally.
    /// Placeholder directories must not be listed: that can start a download.
    pub placeholder: bool,
}

impl EntryMeta {
    /// Converts metadata for `path` that was read without following links
    /// (`symlink_metadata` or `DirEntry::metadata`).
    ///
    /// `path` is only used on Windows, to read what find data lacks.
    #[must_use]
    pub fn from_metadata(path: &Path, md: &Metadata) -> Self {
        let kind = EntryKind::from_file_type(md.file_type());
        let ids = raw_ids(path, md, kind);
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
            placeholder: ids.placeholder,
        }
    }
}

/// Reads metadata for `path` without following a final symlink.
///
/// # Errors
/// Any I/O error from `symlink_metadata`.
pub fn read_meta(path: &Path) -> io::Result<EntryMeta> {
    fs::symlink_metadata(path).map(|md| EntryMeta::from_metadata(path, &md))
}

/// Reads metadata for a directory entry without following links. Same
/// result as [`read_meta`] on the entry's path, but reuses the directory
/// listing where the OS provides it.
///
/// # Errors
/// Any I/O error from `DirEntry::metadata`.
pub fn read_entry_meta(entry: &fs::DirEntry) -> io::Result<EntryMeta> {
    // `DirEntry::metadata` does not follow symlinks on any platform.
    let md = entry.metadata()?;
    // Only Windows needs the path; skip building it elsewhere.
    #[cfg(windows)]
    let path = entry.path();
    #[cfg(not(windows))]
    let path = std::path::PathBuf::new();
    Ok(EntryMeta::from_metadata(&path, &md))
}

struct RawIds {
    allocated: u64,
    dev: u64,
    file_id: u64,
    nlink: u64,
    placeholder: bool,
}

/// `SF_DATALESS` from macOS `<sys/stat.h>`: a dataless (cloud) object.
const SF_DATALESS: u32 = 0x4000_0000;

/// Windows `FILE_ATTRIBUTE_OFFLINE`.
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
/// Windows `FILE_ATTRIBUTE_RECALL_ON_OPEN`.
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
/// Windows `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`.
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

/// Whether macOS `st_flags` mark a dataless (cloud placeholder) object.
#[cfg_attr(
    not(target_os = "macos"),
    allow(dead_code, reason = "used on macOS; tested everywhere")
)]
const fn is_macos_placeholder(st_flags: u32) -> bool {
    st_flags & SF_DATALESS != 0
}

/// Whether Windows file attributes mark data that is not stored locally.
#[cfg_attr(
    not(windows),
    allow(dead_code, reason = "used on Windows; tested everywhere")
)]
const fn is_windows_placeholder(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

#[cfg(unix)]
fn raw_ids(_path: &Path, md: &Metadata, _kind: EntryKind) -> RawIds {
    use std::os::unix::fs::MetadataExt;
    #[cfg(target_os = "macos")]
    let placeholder = {
        use std::os::macos::fs::MetadataExt as _;
        is_macos_placeholder(md.st_flags())
    };
    #[cfg(not(target_os = "macos"))]
    let placeholder = false;
    RawIds {
        // `st_blocks` is always in 512-byte units, whatever the FS block size.
        allocated: md.blocks().saturating_mul(512),
        dev: md.dev(),
        file_id: md.ino(),
        nlink: md.nlink(),
        placeholder,
    }
}

#[cfg(windows)]
fn raw_ids(path: &Path, md: &Metadata, kind: EntryKind) -> RawIds {
    use std::os::windows::fs::MetadataExt;
    let placeholder = is_windows_placeholder(md.file_attributes());
    let mut ids = RawIds {
        allocated: if placeholder { 0 } else { md.len() },
        dev: 0,
        file_id: 0,
        nlink: 1,
        placeholder,
    };
    // Only regular, locally present files are opened: links and junctions
    // are never touched, placeholders could be recalled, and directories
    // need no ids (mount points are reparse points, reported as links).
    if kind != EntryKind::File || placeholder {
        return ids;
    }
    if let Ok(allocated) = filesize::file_real_size_fast(path, md) {
        ids.allocated = allocated;
    }
    // Locked or unreadable files keep the fallbacks: counted once, no dedup.
    if let Ok(info) = open_attributes(path).and_then(|f| winapi_util::file::information(&f)) {
        ids.dev = info.volume_serial_number();
        ids.file_id = info.file_index();
        ids.nlink = info.number_of_links();
    }
    ids
}

/// Opens `path` for attribute queries only, without following reparse points
/// and without read access (so no data is fetched and no recall starts).
#[cfg(windows)]
fn open_attributes(path: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    fs::OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
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
    use super::{EntryKind, is_macos_placeholder, is_windows_placeholder, read_meta};
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
        assert_eq!(f.nlink, 1);
        assert!(!f.placeholder);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hard_links_share_an_id() {
        let dir = std::env::temp_dir().join(format!("cleany-meta-hl-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a");
        let b = dir.join("b");
        fs::write(&a, b"data").unwrap();
        fs::hard_link(&a, &b).unwrap();

        let (ma, mb) = (read_meta(&a).unwrap(), read_meta(&b).unwrap());
        assert_eq!(ma.nlink, 2);
        assert_ne!(ma.file_id, 0);
        assert_eq!((ma.dev, ma.file_id), (mb.dev, mb.file_id));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn placeholder_flags_classified() {
        // Real placeholders cannot be created in a test, so the classifiers
        // are checked against raw flag values.
        assert!(is_macos_placeholder(0x4000_0000));
        assert!(is_macos_placeholder(0x4000_0020)); // dataless + UF_COMPRESSED
        assert!(!is_macos_placeholder(0));
        assert!(!is_macos_placeholder(0x0000_8000)); // UF_HIDDEN

        assert!(is_windows_placeholder(0x0040_0020)); // recall-on-data-access + archive
        assert!(is_windows_placeholder(0x0004_0010)); // recall-on-open + directory
        assert!(is_windows_placeholder(0x0000_1000)); // offline
        assert!(!is_windows_placeholder(0x0000_0020)); // archive
        assert!(!is_windows_placeholder(0x0008_0000)); // pinned
        assert!(!is_windows_placeholder(0x0000_0400)); // reparse point alone
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
