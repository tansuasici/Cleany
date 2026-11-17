---
name: release-signing
description: Finishes the signed Cleany release when the owner says they are ready. Checks the Apple signing secrets, cleans up what the signing setup left behind, and walks the v0.1.0 tag and draft release through, asking before every outward step.
---

# Release Signing

You finish the signed release of Cleany. The owner paused this on 2026-11-17 and will start you when they are ready. Reply to the owner in Turkish, formal “siz”.

## Where things stand
- `.github/workflows/release.yml` builds macOS (universal, signed and notarized when the `APPLE_*` secrets exist), Windows and Linux (unsigned), and uploads them to a draft release. The decision is ADR-0008 in `tasks/decisions.md`.
- The certificate already exists in the owner's login keychain: `Developer ID Application: Tansu Asici (7T387335GA)` (listed twice, both with a private key). Nothing needs to be created on developer.apple.com.
- On 2026-11-17 `APPLE_CERTIFICATE` was set by mistake to an empty value (the `.p12` file did not exist). It must be set again.
- The README says the macOS build is signed and notarized. That is only true once the secrets are in place.

## Never
- Read, type, paste, log or ask for a certificate, `.p12`, password, app-specific password or verification code. The owner runs every `gh secret set` command and every Keychain step themselves. You only give the commands.
- Push to `main`, create or push a tag, publish a release, or delete anything without the owner's explicit yes in chat for that step.
- Bypass a hook.

## Steps
1. **Check the secrets.** Run `gh secret list`. You can see names and update times, never values. The six that must exist: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. If `APPLE_CERTIFICATE` was last updated on 2026-11-17, it is still the empty value: ask the owner to set it again.
2. **Give the owner the commands for what is missing**, each in its own `bash` block:
   - Keychain Access → login → My Certificates → right-click the Developer ID Application certificate → Export… → `.p12` to `~/Desktop/DeveloperID.p12` with a password.
   - `base64 -i ~/Desktop/DeveloperID.p12 | gh secret set APPLE_CERTIFICATE`
   - `gh secret set APPLE_CERTIFICATE_PASSWORD`
   - `gh secret set APPLE_SIGNING_IDENTITY --body "Developer ID Application: Tansu Asici (7T387335GA)"`
   - `gh secret set APPLE_ID`
   - `gh secret set APPLE_PASSWORD` (an app-specific password from account.apple.com → Sign-In and Security → App-Specific Passwords; the owner signs in themselves)
   - `gh secret set APPLE_TEAM_ID --body 7T387335GA`
   Run `gh secret list` again afterwards and confirm all six have today's date.
3. **Clean up.** These steps remove what the signing setup left behind:
   - The exported `.p12` holds the private key. Once `APPLE_CERTIFICATE` is set, ask the owner whether to move `~/Desktop/DeveloperID.p12` to the Trash. Use the Trash, not `rm`, and only on their yes.
   - Check that no `.p12`, `.cer` or base64 dump ended up in the repo: `git status --short` and `git ls-files | grep -iE '\.(p12|cer|pem)$'`. Report anything found and do not delete it yourself.
   - Close any browser tab you opened.
4. **Make sure the release work is on `main`.** `release.yml`, the README and ADR-0008 were prepared on `chore/release-and-readme`. Check whether they are merged (`git log origin/main --oneline -- .github/workflows/release.yml`). If they are not, tell the owner and stop, because a PR needs their yes.
5. **Tag.** Confirm `src-tauri/tauri.conf.json` `version` is `0.1.0`. Ask the owner before you run `git tag v0.1.0 origin/main && git push origin v0.1.0`.
6. **Follow the run.** `gh run list --workflow Release` and `gh run watch`. If macOS fails at signing or notarization, read the log and name the secret that is wrong. Don't guess its value.
7. **Check the draft.** `gh release view v0.1.0` should list a `.dmg`, an `.exe`, an `.msi`, an `.AppImage`, a `.deb` and an `.rpm`. Report the list and ask before you run `gh release edit v0.1.0 --draft=false`.

## Handoff

Before starting, Read `.hook-state/agent-handoff.md` if it exists. Before returning, **overwrite** it with a summary of at most 5 lines: which secrets exist, what was cleaned up, and how far the release got.
