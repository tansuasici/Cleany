# CODEBASE_MAP.md

## What

Cleany is an open-source desktop app (Tauri 2) that shows in detail what uses disk space on macOS, Windows and Linux, lets the user drill into folders, and cleans up safely (trash by default).

## Why

Existing tools are platform-specific, slow on large disks, or unsafe when deleting. Cleany targets fast parallel scanning, correct size accounting (hardlinks, sparse, cloud placeholders), and safe cleanup.

## Tech Stack

- **Language**: Rust (edition 2024, MSRV 1.90 in workspace `Cargo.toml`)
- **Desktop shell**: Tauri 2 (`src-tauri`)
- **Frontend**: Vite + React 19 + TypeScript + Tailwind 4 (`ui/`, pnpm)
- **Async runtime**: Tauri's (tokio) — scanner itself is thread-based (M1)
- **Error handling**: `thiserror` in libs, typed errors to the UI
- **Testing**: `cargo test`; UI: typecheck + oxlint

## Key Commands

| Action | Command |
|--------|---------|
| Dev app | `pnpm --dir ui tauri dev` |
| Bundle | `pnpm --dir ui tauri build` |
| Test | `cargo test` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format | `cargo fmt` |
| CLI | `cargo run -p cleany-cli` |
| UI checks | `pnpm --dir ui typecheck && pnpm --dir ui lint && pnpm --dir ui build` |

## Directory Structure

```text
.
├── Cargo.toml                 # workspace (protected)
├── crates/
│   ├── cleany-core/           # scan model, arena tree, aggregation, analysis
│   ├── cleany-platform/       # OS detection, fast scan paths, trash/reveal
│   └── cleany-cli/            # `cleany` CLI (bench + headless scan)
├── src-tauri/                 # Tauri shell: commands, capabilities, bundle config
├── ui/                        # React frontend (design kit lives here)
├── assets/                    # brand: icon.svg = the main logo (rounded badge, ≥48px), icon-small.svg (≤32px), logo{,-dark,-mono}.svg (bare mark, one-colour use), icon-macos{,-small}.svg (824 plate on 1024, Apple grid; feeds .icns + icon.png); not `tauri icon`
├── docs/PLAN.md               # full plan & milestones
└── .github/workflows/ci.yml   # 3-OS CI
```

## Critical Files

| File | Purpose |
|------|---------|
| `src-tauri/tauri.conf.json` | Window, CSP, bundle targets (protected) |
| `src-tauri/capabilities/default.json` | IPC permissions (protected) |
| `src-tauri/src/lib.rs` | Tauri commands (IPC surface) |
| `ui/src/lib/ipc.ts` | Typed frontend wrapper for commands |
| `crates/cleany-core/src/lib.rs` | Core data types |

## Architecture

UI (React) ⇄ IPC (commands + Channel streams) ⇄ `src-tauri` ⇄ `cleany-core` (tree/aggregation) + `cleany-platform` (OS syscalls). See `docs/PLAN.md` §3.
