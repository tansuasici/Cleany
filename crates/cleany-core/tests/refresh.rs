//! Shallow and subtree refresh against real temp-dir fixtures: after the
//! disk changes, refreshing the changed directories must give the same
//! files (paths, kinds, sizes, counts) as a fresh scan.

#![allow(
    clippy::unwrap_used,
    reason = "fixture helpers outside #[test] fns; a panic is a test failure"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use cleany_core::{
    CancelToken, Depth, NodeId, NodeKind, Outcome, ScanOptions, Size, Tree, refresh_dir, scan,
};
use cleany_platform::read_meta;
use tempfile::TempDir;

/// Per path: kind, the size for non-directories, and the file count. Directory sizes also
/// count the directory's own metadata bytes, which a shallow refresh does
/// not re-read (ADR-0006), so totals are compared through files only.
type Flat = BTreeMap<PathBuf, (NodeKind, Option<Size>, u32)>;

fn write(path: &Path, len: usize) {
    fs::write(path, vec![b'x'; len]).unwrap();
}

fn fixture() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    fs::create_dir_all(r.join("a/keep")).unwrap();
    write(&r.join("a/grow.bin"), 100);
    write(&r.join("a/gone.txt"), 50);
    write(&r.join("a/keep/k.txt"), 7);
    write(&r.join("a/swap"), 3);
    write(&r.join("top.txt"), 9);
    tmp
}

fn fresh(root: &Path) -> Tree {
    scan(root, &ScanOptions::default(), &CancelToken::new(), |_| {})
        .unwrap()
        .tree
}

fn flatten(tree: &Tree) -> Flat {
    fn go(tree: &Tree, id: NodeId, out: &mut Flat) {
        let n = tree.node(id).unwrap();
        let size = (n.kind() != NodeKind::Dir).then(|| n.size());
        out.insert(tree.path(id).unwrap(), (n.kind(), size, n.file_count()));
        for c in tree.children(id) {
            go(tree, c, out);
        }
    }
    let mut out = Flat::new();
    go(tree, tree.root(), &mut out);
    out
}

fn files_total(tree: &Tree) -> (u64, u32) {
    let r = tree.node(tree.root()).unwrap();
    let mut logical = 0;
    for (kind, size, _) in flatten(tree).values() {
        if *kind != NodeKind::Dir {
            logical += size.unwrap().logical;
        }
    }
    (logical, r.file_count())
}

fn refresh(tree: &mut Tree, dir: &Path, depth: Depth) -> Outcome {
    refresh_dir(
        tree,
        dir,
        depth,
        &ScanOptions::default(),
        &CancelToken::new(),
    )
    .unwrap()
    .outcome
}

#[test]
fn shallow_refresh_updates_files_detaches_gone_and_scans_new_dirs() {
    let tmp = fixture();
    let r = tmp.path();
    let mut tree = fresh(r);
    let a = r.join("a");
    let a_id = tree.lookup(&a).unwrap();
    let keep_id = tree.lookup(&a.join("keep")).unwrap();

    write(&a.join("grow.bin"), 4_000);
    fs::remove_file(a.join("gone.txt")).unwrap();
    write(&a.join("new.txt"), 11);
    fs::create_dir_all(a.join("newdir/inner")).unwrap();
    write(&a.join("newdir/inner/n.bin"), 300);
    // Same name, different kind.
    fs::remove_file(a.join("swap")).unwrap();
    fs::create_dir(a.join("swap")).unwrap();
    write(&a.join("swap/s.txt"), 5);

    assert_eq!(
        refresh(&mut tree, &a, Depth::Shallow),
        Outcome::Changed(a_id)
    );
    // Untouched nodes keep their ids.
    assert_eq!(tree.lookup(&a.join("keep")), Some(keep_id));
    let expected = fresh(r);
    assert_eq!(flatten(&tree), flatten(&expected));
    assert_eq!(files_total(&tree), files_total(&expected));
    // Directory totals follow the files: `a` holds grow, new, keep/k,
    // newdir/inner/n and swap/s.
    let logical_files = 4_000 + 11 + 7 + 300 + 5;
    assert!(tree.node(a_id).unwrap().size().logical >= logical_files);
    assert_eq!(tree.node(a_id).unwrap().file_count(), 5);
}

#[test]
fn shallow_refresh_is_idempotent_after_app_trash() {
    let tmp = fixture();
    let r = tmp.path();
    let mut tree = fresh(r);
    let a = r.join("a");
    let a_id = tree.lookup(&a).unwrap();

    // The app moved `gone.txt` to the trash and already detached it.
    let gone = tree.lookup(&a.join("gone.txt")).unwrap();
    tree.detach(gone).unwrap();
    fs::remove_file(a.join("gone.txt")).unwrap();
    let before = flatten(&tree);
    let len = tree.len();

    assert_eq!(refresh(&mut tree, &a, Depth::Shallow), Outcome::Unchanged);
    assert_eq!(refresh(&mut tree, &a, Depth::Shallow), Outcome::Unchanged);
    assert_eq!(flatten(&tree), before);
    assert_eq!(tree.len(), len, "a no-op refresh must not grow the arena");
    assert_eq!(tree.lookup(&a), Some(a_id));
}

#[test]
fn subtree_refresh_replaces_everything_below() {
    let tmp = fixture();
    let r = tmp.path();
    let mut tree = fresh(r);
    let a = r.join("a");
    let a_id = tree.lookup(&a).unwrap();

    write(&a.join("keep/k.txt"), 900);
    fs::create_dir(a.join("keep/more")).unwrap();
    write(&a.join("keep/more/m"), 1);
    fs::remove_file(a.join("gone.txt")).unwrap();

    assert_eq!(
        refresh(&mut tree, &a, Depth::Subtree),
        Outcome::Changed(a_id)
    );
    assert_eq!(tree.lookup(&a), Some(a_id));
    let expected = fresh(r);
    assert_eq!(flatten(&tree), flatten(&expected));
    assert_eq!(files_total(&tree), files_total(&expected));
}

#[test]
fn removed_dir_is_detached_and_removed_root_asks_for_rescan() {
    let tmp = fixture();
    let r = tmp.path();
    let mut tree = fresh(r);
    let a = r.join("a");
    let keep = a.join("keep");

    fs::remove_dir_all(&keep).unwrap();
    assert_eq!(
        refresh(&mut tree, &keep, Depth::Shallow),
        Outcome::Changed(tree.lookup(&a).unwrap())
    );
    assert_eq!(tree.lookup(&keep), None);
    // Already gone from the tree: nothing to do.
    assert_eq!(refresh(&mut tree, &keep, Depth::Shallow), Outcome::Skipped);
    assert_eq!(flatten(&tree), flatten(&fresh(r)));

    let root = r.to_path_buf();
    drop(tmp);
    assert_eq!(refresh(&mut tree, &root, Depth::Shallow), Outcome::RootGone);
}

#[test]
fn refresh_never_enters_another_volume_below_the_root() {
    let tmp = fixture();
    let r = tmp.path();
    let dev = read_meta(r).unwrap().dev;
    // As if the scan root were on another device: every directory below it
    // is then a mount point the first scan recorded but did not enter.
    let other = ScanOptions {
        root_dev: Some(dev.wrapping_add(1)),
        ..ScanOptions::default()
    };
    let mut tree = scan(r, &other, &CancelToken::new(), |_| {}).unwrap().tree;
    let a = r.join("a");
    assert_eq!(tree.children(tree.lookup(&a).unwrap()).count(), 0);
    write(&a.join("new.txt"), 1);
    let before = flatten(&tree);
    let cancel = CancelToken::new();
    for depth in [Depth::Shallow, Depth::Subtree] {
        let done = refresh_dir(&mut tree, &a, depth, &other, &cancel).unwrap();
        assert_eq!(done.outcome, Outcome::Unchanged);
    }
    assert_eq!(flatten(&tree), before);

    // The root itself is always listed; its new subdirectory is recorded but
    // not entered, like in the first scan.
    write(&r.join("root.txt"), 2);
    fs::create_dir(r.join("b")).unwrap();
    write(&r.join("b/inside"), 3);
    let done = refresh_dir(&mut tree, r, Depth::Shallow, &other, &cancel).unwrap();
    assert_eq!(done.outcome, Outcome::Changed(tree.root()));
    assert!(tree.lookup(&r.join("root.txt")).is_some());
    let b = tree.lookup(&r.join("b")).unwrap();
    assert_eq!(tree.children(b).count(), 0);
}
