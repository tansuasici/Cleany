# Security Policy

Cleany reads your whole disk and can move files to the trash, so we take security reports seriously.

## Reporting a vulnerability

**Please do not open a public issue.** Report privately through GitHub:
**Security → [Report a vulnerability](https://github.com/tansuasici/Cleany/security/advisories/new)**.

Include what you can: affected version and OS, steps to reproduce, and the impact you see. We aim to acknowledge reports within 7 days and will keep you updated until a fix is released. With your permission, we credit reporters in the release notes.

## Scope

Especially interesting:

- Deleting or modifying files outside what the user selected (path traversal, symlink/junction following, race conditions)
- Bypassing the protected-path list or the trash-first default
- Tauri IPC / capability / CSP issues that let web content reach the filesystem
- Tampered update or release artifacts

## Supported versions

Cleany is pre-1.0. Only the latest release receives security fixes.
