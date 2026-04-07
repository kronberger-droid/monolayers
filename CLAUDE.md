# Monolayers — File Immutability Daemon

## What This Is

A Cargo workspace producing two binaries that enforce **immutability as the default state** for lab files (WORM — Write Once, Read Many):

- **`monolayers-server`** — runs on the NixOS server, watches `/srv/files`, applies `chattr +i` (kernel-level immutability), serves an HTTP sync API
- **`monolayers-client`** — runs on workstations (Windows/Mac/Linux), syncs files from the server, sets OS-native read-only attributes

The infrastructure (NixOS config, Samba, backup, monitoring) lives in the `monolayers-infra` repo. Design context is in the Obsidian vault at `~/Documents/notes/general-vault/Lab File Storage/`.

## Architecture

```
monolayers-server (NixOS)
├── inotify watcher          → policy engine → chattr +i
├── startup reconciler       → walk files, ensure chattr state
├── sled state store         → tracks which files are locked
└── HTTP sync API (axum)
    ├── GET  /api/manifest   → (path, sha256, size) for all files
    ├── GET  /api/files/*    → download a file
    └── POST /api/files/*    → upload (authenticated)

monolayers-client (Windows/Mac/Linux)
├── sync loop                → poll manifest, diff, download/upload
├── inotify/FSEvents watcher → policy engine → OS read-only attrs
└── sled state store         → tracks local file state
```

Platform-specific code is isolated behind `FilePolicyBackend`:

```rust
trait FilePolicyBackend {
    fn set_readonly(&self, path: &Path, readonly: bool) -> io::Result<()>;
}
```

Implementations:
- **Server:** `ChattrBackend` — `chattr +i` / `chattr -i` (requires `CAP_LINUX_IMMUTABLE`)
- **Client (Linux/Mac):** `ChmodBackend` — clears/sets write bits
- **Client (Windows):** `WindowsBackend` — `SetFileAttributesW` with `FILE_ATTRIBUTE_READONLY` (not yet implemented)

## Workspace Structure

```
Cargo.toml                          (workspace root)
crates/
├── monolayers-core/                (shared library)
│   └── src/
│       ├── backend.rs              FilePolicyBackend trait
│       ├── config.rs               is_exempt() predicate
│       ├── store.rs                StateStore (sled)
│       └── watcher.rs              notify-based recursive watcher
├── monolayers-server/              (server binary)
│   └── src/
│       ├── main.rs                 entry point
│       ├── backend.rs              ChattrBackend
│       ├── config.rs               ServerConfig (TOML)
│       ├── policy.rs               event → chattr policy
│       ├── reconciler.rs           startup walk + enforce
│       └── api.rs                  legacy Nextcloud client (reference)
└── monolayers-client/              (client binary — skeleton)
    └── src/
        └── main.rs
src-legacy/                         old single-binary code (reference)
```

## Sync Protocol

WORM makes sync append-only — no conflict resolution needed:
- **Download:** `server_manifest - local_files`
- **Upload:** `local_files - server_manifest`
- **Collision (same path from two machines):** keep both, rename one

The server daemon serves the sync API directly (axum). No external sync server (Seafile/Nextcloud) required.

## Exempt Folders

Folders matching a configurable name (e.g. `_working`) anywhere in the file tree are excluded from immutability. Files inside are never locked and always writable. The `is_exempt()` predicate is in `monolayers-core` — shared by both binaries.

## Key Dependencies

| Crate | Used by | Purpose |
|-------|---------|---------|
| `tokio` | both | Async runtime |
| `notify` | both | Filesystem watching |
| `sled` | both | Persistent state store |
| `serde` / `toml` | both | Config + serialization |
| `axum` | server | HTTP sync API |
| `reqwest` | server | Legacy Nextcloud API (to be removed) |
| `walkdir` | server | Startup reconciliation |

## Design Decisions

- **`chattr +i` over application-level enforcement** — kernel-enforced, not bypassable without `CAP_LINUX_IMMUTABLE`
- **Non-root daemon** with only `CAP_LINUX_IMMUTABLE` — minimal blast radius
- **Built-in sync** — daemon is the sync server, no Seafile/Nextcloud dependency
- **Two access modes** — SMB for online/LAN, client sync for offline
- **Single .exe client** — cross-compiled via Nix, no installer needed
- **Exempt folders** — `_working` dirs skip immutability, checked by all policy paths
- **Admin = SSH access** — no separate admin UI; write windows via `_working` folders or direct `chattr -i`

## Implementation Progress

### Done
- Workspace restructure (core + server + client)
- `FilePolicyBackend` trait + `ChattrBackend` (server) + `ChmodBackend` (legacy, in core tests)
- `is_exempt()` with tests
- `StateStore` (sled) with tests
- Filesystem watcher
- Server policy engine (event → chattr)
- Server reconciler (walk + enforce)
- Server config (TOML)

### Next Steps
- HTTP sync API (axum: manifest, download, upload)
- Auth for sync API
- Client sync loop
- Windows backend (`SetFileAttributesW`)
- Cross-compilation (`pkgsCross.mingwW64`)
- GitHub Actions for Windows .exe releases
- Error handling (replace `Box<dyn Error>` with proper enum)
- Structured logging (`tracing`)
