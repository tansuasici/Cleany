## What & why
<!-- What does this change and why? Link the issue: Closes #123 -->

## How tested
<!-- Commands run, platforms tried, screenshots for UI changes -->

## Checklist
- [ ] `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass
- [ ] `pnpm --dir ui typecheck`, `lint`, `build` pass (if `ui/` changed)
- [ ] Tests added/updated for behavior changes
- [ ] Touches deletion, capabilities/CSP or the IPC contract? → discussed in an issue first
- [ ] PR title follows Conventional Commits (`feat:`, `fix:`, `docs:` …)
