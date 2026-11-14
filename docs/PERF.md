# Performance baseline (M1)

M1 target (docs/PLAN.md §9): scanning 1M files stays under 150 MB of RAM.

## How to run

```bash
cargo bench -p cleany-core --bench scan
```

- `tree/build_finish_1m`: builds a 1 000 000-file, 10 100-dir in-memory tree
  through `TreeBuilder`, then rolls up the sizes. There is no disk I/O.
- `scan/tempdir_<n>_files`: `scan()` over an on-disk fixture with the
  default thread pool, warm cache. The default is 10 000 files, so that
  `cargo test --all-targets` (CI) runs every bench once quickly. Scale it up
  with `CLEANY_BENCH_FILES=100000 cargo bench ...`.

Peak RSS is measured on the release CLI, because reading it in-process
would need `unsafe`:

```bash
/usr/bin/time -l target/release/cleany scan --json <dir-with-1M-files>   # macOS
/usr/bin/time -v target/release/cleany scan --json <dir-with-1M-files>   # Linux
```

## Results — 2026-11-14

Apple M3 Pro (12 cores), 18 GiB RAM, macOS 27.2, APFS, rustc 1.94.0,
release profile.

| Measurement | Result |
|---|---|
| `tree/build_finish_1m` | 85 ms (80.5–94.8 ms) |
| `scan/tempdir_10000_files` | 12.7 ms (11.4–13.9 ms) |
| CLI scan, 1 000 000 files / 10 101 dirs, warm cache | 3.5–4.5 s wall, about 17 s system CPU |
| CLI scan, 1M files, peak RSS | **58–64 MiB** (target < 150 MB ✅) |

The 1M fixture is the same layout as the synthetic tree: 100 × 100 dirs,
each holding 100 files of 64 bytes with names like
`IMG_20240513_0000042.jpg`.

**Where the time goes:** almost all of it is kernel time spent in
`readdir` and `lstat` (wall time is about 4 s, user CPU about 0.6 s). This is
the cost the M7 fast paths remove: `getattrlistbulk`, `statx` and the NTFS
MFT reader.
