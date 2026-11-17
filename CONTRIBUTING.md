# Contributing to Cleany

Thanks for your interest in Cleany! Bug reports, ideas, docs and code are all welcome.

By participating you agree to follow our [Code of Conduct](CODE_OF_CONDUCT.md).
Security issues: please **do not** open a public issue — see [SECURITY.md](SECURITY.md).

## Before you start

- **Bugs**: open an issue with the bug template (OS, Cleany version, steps to reproduce).
- **Features / larger changes**: open an issue first so we can agree on the approach before you write code.
- **Small fixes** (typos, docs, obvious bugs): a PR is fine directly.

## Development setup

Requirements:

- Rust **1.90+** (`rustup`), with `rustfmt` and `clippy`
- Node **22+** and **pnpm**
- Tauri system dependencies for your OS — see <https://tauri.app/start/prerequisites/>
  (Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`)

```bash
git clone https://github.com/tansuasici/Cleany.git
cd Cleany
pnpm --dir ui install
pnpm --dir ui tauri dev     # run the desktop app with hot reload
```

Other useful commands:

```bash
cargo run -p cleany-cli                   # CLI (shares the scanner core)
pnpm --dir ui tauri build --debug         # local debug bundle
```

### Repository layout

| Path | What |
|---|---|
| `crates/cleany-core` | Platform-independent scanner, tree model, analysis |
| `crates/cleany-platform` | OS-specific fast paths (macOS / Windows / Linux) |
| `crates/cleany-cli` | `cleany` command-line tool |
| `src-tauri/` | Tauri shell: commands, capabilities, bundling |
| `ui/` | React + TypeScript + Vite frontend |

More detail in [CODEBASE_MAP.md](CODEBASE_MAP.md).

## Making a change

1. Fork and create a branch from `main` (`feat/…`, `fix/…`, `docs/…`).
2. Keep the change focused; unrelated refactors go in a separate PR.
3. Add or update tests for behavior changes.
4. Run the checks below — CI runs the same ones on macOS, Windows and Linux.
5. Open a PR and fill in the template.

### Checks

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
pnpm --dir ui typecheck
pnpm --dir ui lint
pnpm --dir ui build
```

### Code rules

- **Rust**: no `.unwrap()` / `.expect()` outside tests — return `Result` and propagate with `?`. `unsafe` is denied workspace-wide; any exception needs discussion and a `// SAFETY:` comment.
- **Never follow symlinks/junctions** in the scanner, and never trigger downloads of cloud placeholder files.
- **Deletion is safety-critical.** Anything touching the delete/trash path, Tauri capabilities/CSP, or the IPC contract needs an issue and maintainer sign-off first.
- New dependencies: explain why in the PR description.

### Commits

We use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `perf:`, `ci:`. PRs are squash-merged, so the PR title becomes the commit message.

## License

Cleany is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Cleany, as defined in the Apache-2.0 license, shall be dual-licensed as above, without any additional terms or conditions.
