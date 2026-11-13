# CLAUDE.project.md — Project Overlay

> This file is never modified by kit upgrades. Add project-specific rules here.
> Rules in this file override kit defaults when there's a conflict.

---

## Project Context
- Cleany is a cross-platform (macOS / Windows / Linux) disk usage analyzer and cleaner built on Tauri 2.
- Full plan, architecture and milestones: `docs/PLAN.md`. Task board: Notion "Cleany" page (kanban).
- Two kits, two session roots:
  - **Backend / Rust / release work** → open the Claude session at the repo root (claude-code-kit, rust template).
  - **UI / design work** → open the Claude session in `ui/` (claude-design-kit; `DESIGN_MAP.md` lives there).
  Hooks only fire for the folder the session was opened in.
- Rust only (ADR-0001). No C/Zig FFI; OS fast paths go through `cleany-platform`.
- Logic lives in `cleany-core` / `cleany-platform`; `src-tauri` commands stay thin.
- Run the Tauri CLI via `pnpm --dir ui tauri <cmd>` (the script `cd`s to the repo root).
- Distribution: GitHub Releases only (dmg, msi/nsis exe, AppImage/deb/rpm). No Mac App Store / MS Store.

---

## Additional Agent Docs
- Plan & architecture → `docs/PLAN.md`

---

## Project-Specific Protected Changes
- Anything that deletes, moves or trashes user files (deletion engine, protected-path list).
- Tauri capabilities (`src-tauri/capabilities/*`) and the CSP in `tauri.conf.json`.
- Bundle identifier `com.tansuasici.cleany` (changing it breaks updates and permissions).
- IPC contract between `src-tauri/src/lib.rs` and `ui/src/lib/ipc.ts`.

---

## Stack-Specific Rules
- Scanner must never follow symlinks/junctions, must respect mount boundaries, and must never hydrate cloud placeholder files.
- Report logical and allocated sizes separately; dedupe hardlinks by (dev, inode) / FileId.
- Delete = move to trash by default; permanent delete needs explicit opt-in and is logged.
- IPC: send tree slices, never whole trees; progress events throttled to ≤10 Hz.
- `unsafe_code` is `deny` at the workspace level; platform FFI that needs it requires an ADR.
