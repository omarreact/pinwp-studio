# PinWP Studio

PinWP Studio is an offline-first desktop workstation for local WordPress development.

## Current MVP

The first milestone focuses on a lightweight Tauri 2 desktop shell with:

- Multi-site registry with create/start/stop lifecycle.
- Per-site local PHP development server when PHP is available.
- SQLite-first and MariaDB-ready environment metadata.
- Runtime preflight for PHP, WP-CLI, Git and Cloudflare Tunnel.
- Dedicated site workspaces for public files, logs, snapshots, exports and protected database files.
- React/TypeScript desktop UI with dark/light appearance.
- Windows CI that builds a native executable artifact.

The MVP is intentionally honest about runtime packaging: the repository does not yet commit third-party PHP, WordPress, SQLite integration, MariaDB or cloudflared binaries. Runtime Packs are the next milestone and are designed to be cached locally for fully offline use after installation.

## Architecture

- Desktop shell: Tauri 2 / Rust
- UI: React + TypeScript + Vite
- Lite database target: official WordPress SQLite Database Integration
- Full compatibility target: MariaDB
- Public development links: Cloudflare Tunnel
- Deployment targets planned: SSH/SFTP, cPanel and a PinWP WordPress connector

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/ROADMAP.md](docs/ROADMAP.md).

## Development

```bash
npm install
npm run dev
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Build a native desktop executable without an installer bundle:

```bash
npm run tauri:build
```
