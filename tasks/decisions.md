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
