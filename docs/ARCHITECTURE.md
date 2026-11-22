# PinWP Studio architecture

## Goal

PinWP Studio is a lightweight, offline-first WordPress desktop workstation. It should create as many isolated local WordPress sites as disk/RAM allow, expose clean local domains, provide optional public development links, and migrate or deploy to ordinary WordPress hosting.

## Runtime modes

### Lite mode

- Native PHP.
- Official WordPress SQLite Database Integration feature plugin.
- Protected SQLite database outside the public document root.
- No database server.
- Intended for theme/plugin/FSE development and lightweight sites.

The SQLite integration requires PDO SQLite and SQLite 3.37+. PinWP Runtime Packs must therefore ship PHP with the required extensions.

### Full compatibility mode

- Native PHP.
- One shared MariaDB service per PinWP installation.
- Separate database per local site.
- Intended for plugins or workloads requiring MySQL/MariaDB-specific behavior.

No Docker container is required for either mode.

## Desktop architecture

```text
Tauri 2 / Rust
├── Site registry
├── Process manager
├── Runtime Pack manager
├── Local domain/router manager
├── Snapshot/import/export engine
├── Tunnel manager
└── Deployment adapters
        ↕
React + TypeScript UI
```

Frontend code never receives unrestricted shell permissions. Native lifecycle operations are implemented as explicit Rust commands with constrained inputs.

## Workspace layout

```text
PinWP Studio/
├── sites.json
├── runtime/
│   ├── php/
│   ├── wordpress/
│   ├── mariadb/
│   └── cloudflared/
└── sites/
    └── my-site/
        ├── public/
        ├── private/
        ├── logs/
        ├── snapshots/
        ├── exports/
        └── .pinwp/site.json
```

A future local router maps `site.wpdev.local` to the assigned loopback port.

## Runtime Packs

Third-party binaries should not be committed casually into Git. Runtime Packs will be built with pinned source/version/checksum metadata and cached locally. An installed Runtime Pack is sufficient for offline operation.

Planned contents: PHP + extensions, WordPress core, SQLite integration, WP-CLI, MariaDB, a local router, and cloudflared.

## Sharing and deployment

Offline access always remains loopback/LAN. Public links are optional and online-only through Cloudflare Tunnel.

Provider-neutral deployment adapters are planned for SSH/SFTP + WP-CLI, cPanel UAPI, a PinWP Connector plugin, and export-only workflows. Production pushes must support backup, URL search/replace, verification and rollback.
