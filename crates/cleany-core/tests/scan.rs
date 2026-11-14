//! Scanner integration tests against real temp-dir fixtures.
//!
//! `reference` is a deliberately naive serial, recursive walker; the parallel
//! scanner must produce exactly the same per-path kinds, sizes and counts.

#![allow(
    clippy::unwrap_used,
    reason = "fixture helpers outside #[test] fns; a panic is a test failure"
)]

use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use cleany_core::{
    CancelToken, NodeFlags, NodeId, NodeKind, ScanError, ScanOptions, Size, Tree, scan,
};
use cleany_platform::{EntryKind, read_meta};
use tempfile::TempDir;

type Flat = BTreeMap<PathBuf, (NodeKind, Size, u32)>;

fn write(path: &Path, len: usize) {
    fs::write(path, vec![b'x'; len]).unwrap();
}

/// Nested dirs, an empty dir, Unicode names and a few hundred small files.
fn fixture() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    fs::create_dir_all(r.join("a/b/c")).unwrap();
    fs::create_dir(r.join("empty")).unwrap();
    write(&r.join("a/one.txt"), 100);
    write(&r.join("a/two.bin"), 5_000);
    write(&r.join("a/b/c/deep.bin"), 70_000);
    write(&r.join("ünïcødé 名前.txt"), 3);
    for d in 0..20 {
        let dir = r.join(format!("many/d{d:02}"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..15 {
            write(&dir.join(format!("f{f}")), d * 15 + f + 1);
        }
    }
    tmp
}

/// Serial reference walk: never follows links, records every path.
fn reference(path: &Path, out: &mut Flat) -> (Size, u32) {
    let meta = read_meta(path).unwrap();
    let kind = NodeKind::from(meta.kind);
    let mut size = Size {
        logical: meta.logical,
        allocated: meta.allocated,
    };
    let mut files = u32::from(kind != NodeKind::Dir);
    if meta.kind == EntryKind::Dir {
        for entry in fs::read_dir(path).unwrap() {
            let (s, f) = reference(&entry.unwrap().path(), out);
            size = size.saturating_add(s);
            files += f;
        }
    }
    out.insert(path.to_path_buf(), (kind, size, files));
    (size, files)
}

fn flatten(tree: &Tree) -> Flat {
    fn go(tree: &Tree, id: NodeId, out: &mut Flat) {
        let n = tree.node(id).unwrap();
        out.insert(tree.path(id).unwrap(), (n.kind(), n.size(), n.file_count()));
        for c in tree.children(id) {
            go(tree, c, out);
        }
    }
    let mut out = Flat::new();
    go(tree, tree.root(), &mut out);
    out
}

fn opts(threads: usize) -> ScanOptions {
    ScanOptions {
        threads: NonZeroUsize::new(threads),
        ..ScanOptions::default()
    }
}

fn scan_ok(root: &Path, options: &ScanOptions) -> cleany_core::ScanResult {
    scan(root, options, &CancelToken::new(), |_| {}).unwrap()
}

#[test]
fn parallel_matches_serial_reference() {
    let tmp = fixture();
    let mut want = Flat::new();
    reference(tmp.path(), &mut want);

    for threads in [1, 4, 0] {
        let res = scan_ok(tmp.path(), &opts(threads));
        assert!(res.issues.is_empty(), "issues: {:?}", res.issues);
        assert_eq!(flatten(&res.tree), want, "threads = {threads}");
        assert_eq!(res.stats.files, 304);
        assert_eq!(res.stats.dirs, 26);
    }
}

#[test]
fn file_root_yields_single_node() {
    let tmp = fixture();
    let file = tmp.path().join("a/two.bin");
    let res = scan_ok(&file, &ScanOptions::default());
    assert_eq!(res.tree.len(), 1);
    let root = res.tree.node(res.tree.root()).unwrap();
    assert_eq!(root.kind(), NodeKind::File);
    assert_eq!(root.size().logical, 5_000);
}

#[test]
fn missing_root_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let err = scan(
        &tmp.path().join("nope"),
        &ScanOptions::default(),
        &CancelToken::new(),
        |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, ScanError::Root { .. }), "{err}");
}

#[test]
fn cancel_stops_scan() {
    let tmp = fixture();
    let cancel = CancelToken::new();
    cancel.cancel();
    let err = scan(tmp.path(), &ScanOptions::default(), &cancel, |_| {}).unwrap_err();
    assert!(matches!(err, ScanError::Cancelled));
}

#[test]
fn hardlinks_counted_once_deterministically() {
    let tmp = fixture();
    let r = tmp.path();
    let target = r.join("a/two.bin");
    let links = [r.join("many/d03/link.bin"), r.join("zz-link.bin")];
    for l in &links {
        fs::hard_link(&target, l).unwrap();
    }
    // The naive walker counts every link in full.
    let mut naive = Flat::new();
    let (naive_total, _) = reference(r, &mut naive);
    let one = naive[&target].1;

    for threads in [1, 4, 0] {
        let res = scan_ok(r, &opts(threads));
        let flat = flatten(&res.tree);
        assert_eq!(flat[&target].1, one, "smallest path owns the bytes");
        for l in &links {
            assert_eq!(flat[l].1, Size::default(), "{}", l.display());
        }
        let root = res.tree.node(res.tree.root()).unwrap();
        assert_eq!(
            root.size().allocated,
            naive_total.allocated - 2 * one.allocated
        );
        assert_eq!(root.file_count(), 306, "links still count as files");
        assert_eq!(res.stats.hardlinks, 2, "threads = {threads}");
        let flagged = res
            .tree
            .children(res.tree.root())
            .filter(|&c| {
                res.tree
                    .node(c)
                    .unwrap()
                    .flags()
                    .contains(NodeFlags::HARDLINK)
            })
            .count();
        assert_eq!(flagged, 1, "only zz-link.bin at the top level");
    }
}

#[cfg(unix)]
#[test]
fn symlink_loop_is_not_followed() {
    use std::os::unix::fs::symlink;
    let tmp = fixture();
    symlink("..", tmp.path().join("a/b/up")).unwrap();
    symlink(tmp.path(), tmp.path().join("a/self")).unwrap();

    let mut want = Flat::new();
    reference(tmp.path(), &mut want);
    let res = scan_ok(tmp.path(), &ScanOptions::default());
    assert_eq!(flatten(&res.tree), want);
    assert_eq!(res.stats.symlinks, 2);
    let up = &want[&tmp.path().join("a/b/up")];
    assert_eq!(up.0, NodeKind::Symlink);
}

#[cfg(unix)]
#[test]
fn symlink_root_is_resolved_once() {
    let tmp = fixture();
    let outer = tempfile::tempdir().unwrap();
    let link = outer.path().join("root-link");
    std::os::unix::fs::symlink(tmp.path(), &link).unwrap();

    let direct = scan_ok(tmp.path(), &ScanOptions::default());
    let via = scan_ok(&link, &ScanOptions::default());
    let size = |r: &cleany_core::ScanResult| r.tree.node(r.tree.root()).unwrap().size();
    assert_eq!(size(&via), size(&direct));
    assert_eq!(via.tree.path(via.tree.root()).unwrap(), link);
}

#[cfg(unix)]
#[test]
fn permission_denied_is_reported_and_scan_continues() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = fixture();
    let locked = tmp.path().join("locked");
    fs::create_dir(&locked).unwrap();
    write(&locked.join("hidden"), 10);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let readable = fs::read_dir(&locked).is_ok(); // true when running as root
    let res = scan_ok(tmp.path(), &ScanOptions::default());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        eprintln!("skipping: running with permission to read a 000 directory");
        return;
    }

    assert_eq!(res.issues.len(), 1, "{:?}", res.issues);
    assert_eq!(res.issues[0].path, locked);
    assert_eq!(
        res.issues[0].error.kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        res.stats.files, 304,
        "the rest of the tree is still scanned"
    );
}

#[test]
fn sparse_file_allocated_less_than_logical() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("sparse.img");
    let f = fs::File::create(&path).unwrap();
    f.set_len(64 * 1024 * 1024).unwrap();
    drop(f);
    let meta = read_meta(&path).unwrap();
    if meta.allocated >= meta.logical {
        eprintln!("skipping: filesystem does not create sparse files");
        return;
    }

    let res = scan_ok(tmp.path(), &ScanOptions::default());
    let flat = flatten(&res.tree);
    let (_, size, _) = flat[&path];
    assert_eq!(size.logical, 64 * 1024 * 1024);
    assert_eq!(size.allocated, meta.allocated);
    assert!(size.allocated < size.logical);
}

#[cfg(windows)]
#[test]
fn junction_is_not_followed() {
    let tmp = fixture();
    let link = tmp.path().join("junction");
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(tmp.path().join("a"))
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        eprintln!("skipping: could not create a junction");
        return;
    }

    let res = scan_ok(tmp.path(), &ScanOptions::default());
    let flat = flatten(&res.tree);
    assert_eq!(flat[&link].0, NodeKind::Symlink);
    assert!(!flat.contains_key(&link.join("one.txt")));
    assert_eq!(res.stats.symlinks, 1);
}
