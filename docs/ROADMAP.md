# PinWP Studio roadmap

## Milestone 0 — Desktop foundation (this PR)
- Tauri 2 + React/TypeScript desktop shell.
- Multi-site registry.
- Create local workspaces.
- Start/stop PHP server when PHP exists.
- Runtime preflight.
- Dark/light responsive UI.
- Windows native executable CI artifact.

## Milestone 1 — Offline Runtime Pack
- Signed/checksummed Runtime Pack manifest.
- Bundled PHP for Windows first.
- WordPress core cache.
- Official SQLite Database Integration provisioning.
- Protected DB_PATH.
- One-click WordPress installation.
- WP-CLI bundled/cached.
- Existing site import.

Acceptance: a clean Windows machine can create and run WordPress without separately installing PHP/MySQL/Docker and without Internet after PinWP + its Runtime Pack are installed.

## Milestone 2 — Local routing and HTTPS
- Shared router, .wpdev.local domains, local certificates, health checks, LAN share and QR.

## Milestone 3 — Full compatibility
- Shared MariaDB, per-site databases/users, export workflow, PHP/WP version manager.

## Milestone 4 — Developer tooling
- Snapshots/restore, mail catcher, logs, DB browser, editor launch, WP-CLI, Git.

## Milestone 5 — Share
- Cloudflare Quick Tunnel, protected previews, persistent tunnels.

## Milestone 6 — Migrate / deploy
- Full export, SSH/SFTP, cPanel UAPI, PinWP Connector, backup/rollback.

## Milestone 7 — Compatibility matrix
- Disposable clones, WordPress × PHP matrix, smoke tests and screenshots.
