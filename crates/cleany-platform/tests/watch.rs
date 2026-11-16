//! Real FSEvents round trips on macOS: a change below a watched temp dir is
//! reported in the caller's spelling of the root (temp dirs live under the
//! `/var` -> `/private/var` symlink), and nothing arrives after the drop.

#![cfg(target_os = "macos")]

use std::fs;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use cleany_platform::{ChangeKind, FsChange, watch};

const TIMEOUT: Duration = Duration::from_secs(10);

/// Waits until `want` arrives or the timeout passes.
fn wait_for(rx: &Receiver<FsChange>, want: impl Fn(&FsChange) -> bool) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(change) if want(&change) => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

#[test]
fn reports_created_file_and_removed_dir_under_the_given_root() {
    let tmp = tempfile::tempdir().unwrap();
    // Usually `/var/folders/...`, which FSEvents reports as `/private/var/...`.
    let root = tmp.path();
    fs::create_dir(root.join("sub")).unwrap();
    let (tx, rx) = channel();
    let watcher = watch(root, move |change| {
        let _ = tx.send(change);
    })
    .unwrap();

    let file = root.join("new.txt");
    fs::write(&file, b"hello").unwrap();
    assert!(
        wait_for(&rx, |c| c.path == file && c.kind == ChangeKind::Entry),
        "no event for {}",
        file.display()
    );

    let sub = root.join("sub");
    fs::remove_dir(&sub).unwrap();
    assert!(
        wait_for(&rx, |c| c.path == sub),
        "no event for {}",
        sub.display()
    );

    // Everything reported stays under the caller's root.
    drop(watcher);
    while let Ok(change) = rx.try_recv() {
        assert!(change.path.starts_with(root), "{}", change.path.display());
    }
    // The handler (and its sender) is gone with the watcher.
    fs::write(root.join("after.txt"), b"x").unwrap();
    assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());
}

#[test]
fn missing_root_fails_to_start() {
    let tmp = tempfile::tempdir().unwrap();
    let gone = tmp.path().join("gone");
    assert!(watch(Path::new(&gone), |_| {}).is_err());
}
