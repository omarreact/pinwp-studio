# PinWP Studio v1 — product status

PinWP Studio is a Windows-first, offline-first WordPress development workstation.

## Included in v1

- Tauri 2 native desktop app with React/TypeScript UI.
- Lite WordPress mode: bundled PHP + official SQLite Database Integration.
- Full WordPress mode: bundled PHP + MariaDB with isolated per-site databases/users.
- Multi-site lifecycle with zero-config `*.localhost:<port>` URLs.
- Optional `*.wpdev.local` trusted HTTPS through bundled Caddy. Windows administrator approval is required for hosts/CA trust.
- Snapshots and restore for files + SQLite/MariaDB.
- Portable `.pinwp.zip` export/import.
- SQLite and MariaDB query console with write SQL gated by an explicit checkbox.
- PHP/WordPress logs.
- WP-CLI runner with eval/shell commands gated by Unsafe mode.
- Git init/status/commit and VS Code launcher when those host tools are installed.
- Mailpit local mail catcher.
- Cloudflare Quick Tunnel public preview links.
- Compatibility check: PHP version, WordPress version, core checksums, plugin/theme counts.
- Deployment adapters:
  - SSH/SCP: pre-deploy remote file backup + upload/extract.
  - cPanel UAPI: authenticated package upload.
  - PinWP Connector: authenticated WordPress REST file deployment with pre-deploy file backup.
- Pinned/checksummed Windows Runtime Pack and portable build workflows.

## Security model

The frontend never receives unrestricted shell access. Native operations are explicit Tauri commands. Deployment secrets are supplied per action and are not persisted in the site registry. Database mutation is blocked by default. WP-CLI eval/shell is blocked unless Unsafe mode is explicitly enabled.

## Known boundaries

- PinWP Connector v1 intentionally does not auto-apply database SQL on remote WordPress hosts.
- cPanel v1 uploads the verified deployment package; server-side extraction/application depends on cPanel/server capabilities and is not forced automatically.
- SSH deployment assumes the remote server has `unzip`; database migration remains a separate production concern.
- Trusted local HTTPS requires Windows UAC because the hosts file and trust store are privileged.
- Git, SSH and VS Code integration use host-installed tools when available.
