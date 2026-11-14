//! End-to-end checks of the `cleany` binary, including a cross-check of the
//! allocated total against `du` on Unix.

#![allow(
    clippy::unwrap_used,
    reason = "fixture helpers outside #[test] fns; a panic is a test failure"
)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn cleany(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cleany"))
        .args(args)
        .output()
        .unwrap()
}

/// A few MiB spread over nested dirs, so per-entry rounding stays well under 1%.
fn fixture(root: &Path) {
    for d in 0..8 {
        let dir = root.join(format!("d{d}/sub"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..12 {
            let len = 4_000 + d * 37_000 + f * 9_100;
            fs::write(dir.join(format!("f{f}.bin")), vec![b'x'; len]).unwrap();
        }
    }
    fs::create_dir(root.join("empty")).unwrap();
    fs::write(root.join("big.bin"), vec![b'y'; 2 * 1024 * 1024]).unwrap();
}

fn scan_json(root: &Path) -> Value {
    let out = cleany(&["scan".as_ref(), "--json".as_ref(), root.as_os_str()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn json_report_has_totals_and_children() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let v = scan_json(tmp.path());
    assert_eq!(v["files"], 97);
    assert_eq!(v["dirs"], 18);
    assert!(v["allocated"].as_u64().unwrap() > 0);
    let kids = v["children"].as_array().unwrap();
    assert_eq!(kids.len(), 10);
    // Sorted largest first.
    let first = kids[0]["allocated"].as_u64().unwrap();
    assert!(
        kids.iter()
            .all(|k| k["allocated"].as_u64().unwrap() <= first)
    );
    assert!(v["issues"].as_array().unwrap().is_empty());
    assert_eq!(v["hardlinks"], 0);
    assert_eq!(v["placeholders"], 0);
}

#[test]
fn missing_path_fails_and_usage_errors_exit_2() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("nope");
    let out = cleany(&["scan".as_ref(), missing.as_os_str()]);
    assert_eq!(out.status.code(), Some(1));

    let out = cleany(&["scan".as_ref()]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn text_report_lists_largest_child() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let out = cleany(&["scan".as_ref(), tmp.path().as_os_str()]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("97 files, 18 dirs"), "{text}");
    assert!(text.contains("big.bin"), "{text}");
}

#[cfg(unix)]
#[test]
fn du_total_matches_within_1_percent() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    // `du` counts each hard-linked file once; so must we.
    fs::hard_link(
        tmp.path().join("big.bin"),
        tmp.path().join("d0/big-link.bin"),
    )
    .unwrap();

    // POSIX `du -s -k`: total allocated KiB, directories included.
    let du = Command::new("du").arg("-sk").arg(tmp.path()).output();
    let Ok(du) = du.map_err(|e| eprintln!("skipping: cannot run du: {e}")) else {
        return;
    };
    assert!(du.status.success());
    let du_kib: u64 = String::from_utf8(du.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();

    let ours = scan_json(tmp.path())["allocated"].as_u64().unwrap();
    let du_bytes = du_kib * 1024;
    let diff = ours.abs_diff(du_bytes);
    assert!(
        diff * 100 <= du_bytes,
        "cleany {ours} B vs du {du_bytes} B (diff {diff})"
    );
}
