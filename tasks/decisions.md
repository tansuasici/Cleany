# Architecture Decision Records

Track important technical decisions here so they don't get lost between sessions.

---

## Format

```markdown
### ADR-[number]: [Short title]
- **Date**: YYYY-MM-DD
- **Status**: proposed | accepted | rejected | superseded
- **Context**: [What problem are we solving? What constraint exists?]
- **Options**:
  - A) [Option] — Pros: ... / Cons: ...
  - B) [Option] — Pros: ... / Cons: ...
- **Decision**: [Which option and why]
- **Consequences**: [What changes as a result? Any risks?]
```

---

## Example

### ADR-001: Use Zod for request validation
- **Date**: 2026-03-01
- **Status**: accepted
- **Context**: API endpoints accept user input without validation. Need runtime validation with TypeScript type inference.
- **Options**:
  - A) Zod — Pros: TypeScript-first, small bundle, great DX / Cons: another dependency
  - B) Joi — Pros: mature, battle-tested / Cons: no TS inference, larger
  - C) Manual validation — Pros: no dependency / Cons: error-prone, verbose
- **Decision**: Zod (A). TypeScript inference eliminates duplicate type definitions. Small enough to justify the dependency.
- **Consequences**: All route handlers must validate input with Zod schemas. Schemas live in `src/schemas/`.

---

<!-- Add new decisions below this line -->

### ADR-0001: Rust only for the backend
- **Date**: 2026-11-13
- **Status**: accepted
- **Context**: Scanner speed is bound by filesystem syscalls, not CPU. Tauri is Rust-native.
- **Options**:
  - A) Rust only — Pros: one toolchain, memory safety, direct OS APIs via crates / Cons: none significant
  - B) Rust + Zig/C scanner over FFI — Pros: none measurable / Cons: second toolchain, unsafe FFI boundary, harder CI
- **Decision**: A. See `docs/PLAN.md` §1.
- **Consequences**: OS fast paths (getattrlistbulk, statx, FindFirstFileExW, MFT) live in `cleany-platform`.

### ADR-0002: License MIT OR Apache-2.0
- **Date**: 2026-11-13
- **Status**: accepted
- **Context**: Open-source project; Rust ecosystem convention.
- **Decision**: Dual license MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
- **Consequences**: Contributions are accepted under both licenses.

### ADR-0003: Distribute via GitHub Releases only
- **Date**: 2026-11-13
- **Status**: accepted
- **Context**: A disk cleaner needs full-disk access; App Store sandboxing blocks it.
- **Decision**: tauri-action publishes dmg (macOS), msi + NSIS exe (Windows), AppImage/deb/rpm (Linux) to GitHub Releases. No Mac App Store / MS Store.
- **Consequences**: Signing/notarization handled in our release workflow; updater uses GitHub Releases.

### ADR-0004: Click drills in; deletion is never a single click
- **Date**: 2026-11-13
- **Status**: accepted
- **Context**: "Clicking an item" could mean drilling into it or deleting it. A one-click delete in a disk cleaner is a data-loss risk.
- **Options**:
  - A) Click drills in — Pros: safe, matches treemap/sunburst navigation / Cons: deletion needs extra steps
  - B) Click deletes — Pros: fastest cleanup / Cons: accidental data loss, no undo for permanent delete
- **Decision**: A. Single click on a folder drills in (breadcrumb to go back); on a file it selects and shows details. Deletion is select → basket → "Move to Trash" confirmation.
- **Consequences**: M3 views implement drill-down + breadcrumb; M4 owns the selection basket and confirmation flow. Deletion engine stays a protected change.

### ADR-0005: cleany-core depends on cleany-platform; platform returns raw metadata
- **Date**: 2026-11-14
- **Status**: accepted
- **Context**: The scanner (core) needs OS-specific metadata: allocated size, (dev, inode) / FileId, link count, reparse and cloud-placeholder flags. Shared types like `Size` and the arena tree live in core. If both crates depended on each other, that would be a cycle.
- **Options**:
  - A) core → platform; platform returns plain `u64`/flag fields (`EntryMeta`) — Pros: platform stays a thin syscall layer, core owns the model / Cons: platform can't use core types
  - B) platform → core; platform builds `Size`/nodes itself — Pros: fewer conversions / Cons: OS code leaks into the model, harder to test core in isolation
  - C) a trait in core implemented by platform — Pros: mockable / Cons: indirection with only one real implementation
- **Decision**: A. `cleany-platform` exposes `EntryMeta` built from `std::fs::Metadata` (M1) and later the fast paths (M7). `cleany-core` converts it into tree entries.
- **Consequences**: Platform must not depend on core. Windows-only helpers in M1 use safe wrapper crates (`winapi-util`, `filesize`), so `unsafe_code` stays denied. Revisit for the M7 MFT path.

### ADR-0006: Live updates — FSEvents, shallow refresh, stable ids, no on-disk cache
- **Date**: 2026-11-16
- **Status**: accepted
- **Context**: A finished scan goes stale as soon as files change, including changes the app makes itself (move to Trash). A full rescan of a large disk takes minutes, and it replaces every `NodeId` that the UI holds in its selection basket, breadcrumb and open views.
- **Options**:
  - A) Rescan the root on every change — Pros: trivially correct / Cons: minutes of I/O per change, every id the UI holds is lost
  - B) Watch the root (FSEvents / `notify`), fold events into directories, refresh those directories in place — Pros: work grows with the change, not the disk; untouched ids stay valid / Cons: patch logic in core, sizes can drift where events are lost
  - C) B plus an on-disk cache to survive restarts — Pros: instant reopen / Cons: invalidation across reboots and unmounts, a second source of truth, format versioning
- **Decision**: B, without a cache. `cleany-core::refresh` folds events with a `Coalescer` (1 s quiet window, 5 s max delay, more than 256 changed siblings escalate to a rescan of their parent, more than 10 000 pending directories fall back to a full rescan). Each directory is refreshed in three steps so the session lock is never held during I/O: `request` (read the tree), `read` (list the disk, scan new subdirectories with the original `root_dev`), `apply` (patch the tree). A shallow refresh matches children by name and kind: files are updated in place, vanished entries are detached, new files are appended, new directories are scanned and grafted, existing directories are left to their own events. A dropped-event signal rescans that subtree. The watcher (`notify`) and the session/IPC wiring follow in later PRs.
- **Consequences**: `Tree` gains `lookup`, `update_entry`, `add_child`, `graft` and `graft_children`; detached slots are never reused, so the arena grows with churn until the next full scan. Patches are idempotent, so the app's own trash moves refresh to a no-op. A directory's own metadata size (bytes of its entry table) is not re-read by a shallow refresh; that drift is bounded by a few KiB per directory and disappears at the next full scan. Paths are matched exactly as scanned, so the watcher must report paths under the same spelling as the scan root (firmlinks and symlinked roots need normalising in the watcher).
- **Watcher (PR 3, 2026-11-16)**: `cleany-platform::watch` wraps `notify` 8.2.2 (stable; macOS-only target dependency, default `macos_fsevent`, CC0-1.0). It reports `FsChange { path, kind: Entry | Subtree }`; dropped events (`MustScanSubDirs`) and stream errors become `Subtree`, reads and owner/permission/xattr/Finder-info updates are dropped as noise. FSEvents reports resolved paths (`/private/var/...`), which are mapped back under the caller's root. Measured: watching `/` reports firmlinked paths as `/Users/...`, not `/System/Volumes/Data/...`, so no firmlink mapping is needed for a `/` scan; also measured: a root spelled `/System/Volumes/Data` receives no events (that volume is hidden from the picker). Other OSes return `WatchError::Unsupported`. Known risk, decision pending with the owner: `notify` 8.2.2 panics in its FSEvents callback on an unknown event flag or a path that is not valid UTF-8, and `panic = "abort"` turns that into an app exit. Its flag set matches all 24 flags in the macOS 27.2 SDK; APFS/HFS+ names are UTF-8, but network volumes (NFS/SMB) may not be. `notify` 9.0.0-rc fixes both (truncates unknown flags, keeps raw path bytes). `sinceWhen` replay is not exposed, which matters only for a future on-disk cache.
- **Session and UI (PR 4, 2026-11-16)**: each scan starts a watcher at the same time as the scan (events during the scan are buffered and applied after `done`, so nothing is missed between the walk and the watch) and a `cleany-live-N` thread that owns the `Coalescer`. Events go to a per-scan sink shared by the scan thread and the live thread; the scan thread holds it across `done`, so `changed` never precedes `done`, and `attach_scan` replaces the sink so a reloaded webview keeps receiving updates. A refresh reads the disk without the session lock and is applied only if no trash move happened in between (an `edits` counter); a stale read is retried up to 3 times. A change below an unknown directory refreshes the nearest known ancestor shallowly. `changed { scan, revision, root }` follows every round that changed the tree; `outdated { scan }` stops live updates when the root is gone, a patch fails, the coalescer asks for a full rescan, or the tree grew past `max(2 × scanned, scanned + 100 000)` nodes. A new scan drops the previous watcher. The UI keeps the previous data on screen while each view reads the new revision, follows the open folder to its nearest surviving ancestor, refreshes the basket and the döküm, and shows a rescan notice on `outdated`.

### ADR-0007: Webview permissions — empty capability list, strict CSP
- **Date**: 2026-11-17
- **Status**: accepted
- **Context**: Cleany reads the whole disk and can move files to the Trash, so script running in the webview is the most valuable thing to attack. The default capability (`core:default`) opened the path, event, window, webview, app, image, resources, menu and tray APIs, none of which the UI calls: it uses only `invoke` and `Channel` from `@tauri-apps/api/core`. The CSP allowed the unused `asset:` protocol, inline `<style>` and left `object-src`, `base-uri` and `form-action` open.
- **Options**:
  - A) Keep `core:default` — Pros: nothing to maintain / Cons: an XSS gets window, menu, tray and path APIs for free
  - B) Empty permission list, add one narrow permission per real need — Pros: smallest surface, each permission is a reviewed decision / Cons: a new core API call fails until its permission is added
- **Decision**: B. `capabilities/default.json` grants nothing to `main`. The app's own commands are not gated by capabilities (no app manifest in `build.rs`), and channel messages go through Tauri's internal channel plugin, which needs no permission. CSP: `default-src 'self'; script-src 'self'; style-src 'self'; style-src-attr 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; worker-src 'none'`. `style-src-attr 'unsafe-inline'` covers React `style={{…}}` (bar widths, transforms); no inline `<style>` elements are allowed. `freezePrototype: true` stops prototype pollution from reaching the IPC layer.
- **Consequences**: Any future use of a core API or plugin from the UI must add its narrowest permission here, with the reason, as a protected change. The dev app (`tauri dev`, served by Vite with injected styles and HMR) kept working unchanged after this change, so CSP regressions are reliably caught only in a release build, which is the place to test them. Loading a remote resource, a web font or a worker needs a CSP change first.

### ADR-0008: Release builds — Tauri CLI on three runners, draft GitHub Release
- **Date**: 2026-11-17
- **Status**: accepted
- **Context**: v0.1.0 needs installers for macOS, Windows and Linux. A Mac cannot build the Windows and Linux bundles, and a local macOS build is ad-hoc signed and arm64 only. The owner chose: macOS signed and notarized, Windows and Linux unsigned.
- **Options**:
  - A) `release.yml` runs `tauri build` itself on each runner and uploads with `gh release upload` — Pros: every step visible in the file, same toolchain setup as `ci.yml`, no third-party action with write access / Cons: asset globs and draft creation maintained by hand
  - B) `tauri-apps/tauri-action` — Pros: less YAML, handles the release / Cons: an extra action holding `contents: write`, its behaviour changes with its version
- **Decision**: A. A `v*` tag (or a manual run with a tag) first checks that the tag matches `tauri.conf.json` `version`, then creates a draft release with generated notes. A matrix builds macOS as `universal-apple-darwin` (signed and notarized through the `APPLE_*` secrets, which the Tauri CLI reads), Windows (NSIS and MSI) and Linux (AppImage, deb, rpm) and uploads them to the draft. Publishing the draft stays a manual step.
- **Consequences**: The owner adds `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` as repository secrets. Without them the macOS build falls back to an unsigned bundle. Windows users see SmartScreen until code signing is added. Bumping the version means changing `tauri.conf.json` before tagging.
