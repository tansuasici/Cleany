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
