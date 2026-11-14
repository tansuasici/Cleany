//! Benchmarks for the arena tree and the scanner (docs/PLAN.md §9).
//!
//! - `tree/build_finish_1m`: builds and rolls up a synthetic in-memory tree of
//!   1 000 000 files (10 100 dirs) with realistic ~24-byte names. No disk I/O.
//! - `scan/tempdir`: scans an on-disk fixture with the default thread pool.
//!   The fixture holds 10 000 files by default, so `cargo test --all-targets`
//!   (which runs every bench once) stays fast on CI. Set
//!   `CLEANY_BENCH_FILES` to scale it locally. Timings are warm-cache.
//!
//! Peak RSS of a real 1M-file scan is measured on the release CLI instead
//! (see `docs/PERF.md`), since reading it in-process would need `unsafe`.

#![allow(
    clippy::unwrap_used,
    reason = "bench fixtures; a panic is a bench failure"
)]

use std::ffi::OsString;
use std::fs;
use std::hint::black_box;
use std::path::Path;

use cleany_core::{
    CancelToken, EntryInfo, NodeFlags, NodeKind, ScanOptions, Size, Tree, TreeBuilder, scan,
};
use criterion::{Criterion, criterion_group, criterion_main};

const TOP: usize = 100;
const SUB: usize = 100;
const FILES: usize = 100;

fn info(kind: NodeKind, bytes: u64) -> EntryInfo {
    EntryInfo {
        kind,
        size: Size {
            logical: bytes,
            allocated: bytes.next_multiple_of(4096),
        },
        mtime: 1_700_000_000,
        flags: NodeFlags::NONE,
    }
}

/// `TOP * SUB` directories holding `FILES` files each: 1 000 000 files.
fn build_synthetic() -> Tree {
    let mut b = TreeBuilder::new("/synthetic".as_ref(), info(NodeKind::Dir, 4096)).unwrap();
    let root = b.root();
    let mut name = OsString::new();
    for t in 0..TOP {
        name.clear();
        name.push(format!("project-module-{t:03}"));
        let top = b.add_child(root, &name, info(NodeKind::Dir, 4096)).unwrap();
        for s in 0..SUB {
            name.clear();
            name.push(format!("src-component-{s:03}"));
            let sub = b.add_child(top, &name, info(NodeKind::Dir, 4096)).unwrap();
            for f in 0..FILES {
                let n = (t * SUB + s) * FILES + f;
                name.clear();
                name.push(format!("IMG_20240513_{n:07}.jpg"));
                b.add_child(sub, &name, info(NodeKind::File, (n % 9_000) as u64 + 100))
                    .unwrap();
            }
        }
    }
    b.finish()
}

fn bench_tree(c: &mut Criterion) {
    let mut g = c.benchmark_group("tree");
    g.sample_size(10);
    g.bench_function("build_finish_1m", |bench| {
        bench.iter(|| black_box(build_synthetic()));
    });
    g.finish();
}

/// `files` small files, 100 per directory, two directory levels deep.
fn disk_fixture(root: &Path, files: usize) {
    let dirs = files.div_ceil(FILES);
    for d in 0..dirs {
        let dir = root.join(format!("module-{:03}/component-{d:05}", d % 50));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..FILES.min(files - d * FILES) {
            fs::write(dir.join(format!("source_file_{f:04}.rs")), [b'x'; 64]).unwrap();
        }
    }
}

fn bench_scan(c: &mut Criterion) {
    let files = std::env::var("CLEANY_BENCH_FILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let tmp = tempfile::tempdir().unwrap();
    disk_fixture(tmp.path(), files);

    let mut g = c.benchmark_group("scan");
    g.sample_size(10);
    g.bench_function(format!("tempdir_{files}_files"), |bench| {
        bench.iter(|| {
            let r = scan(
                tmp.path(),
                &ScanOptions::default(),
                &CancelToken::new(),
                |_| {},
            )
            .unwrap();
            black_box(r.tree.len())
        });
    });
    g.finish();
}

criterion_group!(benches, bench_tree, bench_scan);
criterion_main!(benches);
