# Monolayers — Nextcloud File Persistence Daemon

## What This Is

A Windows daemon that enforces **immutability as the default state** for files in Nextcloud. Files are read-only by default (including for the owner) and can only be written during explicitly opened write windows controlled by an admin.

The daemon does **not** handle sync — the Nextcloud desktop client does all file transfer. The daemon's sole job is keeping local filesystem permissions in sync with server-side tag state.

## Core Mechanism

**Restricted system tags** via `files_access_control` app — not file locks. A restricted tag (`immutable`, `userAssignable: false`) is applied to files; only the admin (daemon) can add/remove it. The File Access Control rule denies writes when the tag is present.

### Tag API (WebDAV, admin credentials)

- **Create tag:** `POST /remote.php/dav/systemtags` with `{"name": "immutable", "userVisible": true, "userAssignable": false}`
- **Apply tag:** `PUT /remote.php/dav/systemtags-relations/files/{fileId}/{tagId}`
- **Remove tag:** `DELETE /remote.php/dav/systemtags-relations/files/{fileId}/{tagId}`
- **Query tagged files:** `REPORT /remote.php/dav/files/admin/` with `<oc:systemtag>{tagId}</oc:systemtag>` filter

## Architecture

```
tokio runtime
├── task: CfApi listener      →  open/close events     →  policy engine
├── task: notify watcher      →  NC client write events →  policy engine
├── task: policy engine       →  is_exempt(path)?
│                                  no  → tag API + set ACL RO
│                                  yes → ensure untagged + ACL RW
└── task: startup reconciler  →  REPORT query → walk local files → apply policy
```

Platform-specific code is isolated behind a `FilePolicyBackend` trait:

```rust
trait FilePolicyBackend {
    async fn on_open(&self, path: &Path);
    async fn on_close(&self, path: &Path);
    fn set_readonly(&self, path: &Path, readonly: bool) -> io::Result<()>;
}
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `reqwest` | WebDAV/OCS API calls |
| `serde` | Serialization |
| `quick-xml` | WebDAV XML parsing |
| `notify` | Filesystem watching (`ReadDirectoryChangesW`) |
| `windows` | CfApi + `SetFileAttributesW` |
| `sled` | Persistent local state (`path → fileId → tag_state`) |

## Design Decisions

- **Windows only** — CfApi provides native file open/close callbacks when NC client runs in VFS mode.
- **One NC user per machine** — no multi-client coordination needed; each daemon manages only its own user's files.
- **Mobile clients get read-only share access** — no daemon needed on mobile.
- **Write windows are admin-only** — initiated via Nextcloud web UI by removing the restricted tag. No desktop/mobile client can open one.
- **Exempt folders** — folders matching a configurable name (e.g. `_working`) anywhere in the sync tree are excluded from the immutability policy. Files inside are never tagged and always writable.

## Exempt Folder Path Predicate

```rust
fn is_exempt(path: &Path, exempt_names: &[&str]) -> bool {
    path.ancestors().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| exempt_names.contains(&n))
            .unwrap_or(false)
    })
}
```

All policy application — tagging, ACL, startup reconciliation, move handling — checks this first.

## State Management

- Daemon is the sole writer of server tags → its own source of truth.
- Local state in `sled`: `local_path → (fileId, tag_state)`.
- On restart: reconcile local state against server REPORT query.

## Permission Mapping

| State | Server | Local (Windows) |
|-------|--------|-----------------|
| Immutable (default) | Tag present, writes denied | ACL deny write |
| Write window open | Tag absent, writes allowed | ACL allow write |
| Exempt folder | No tag applied | ACL allow write |

## Implementation Progress

### Done
- **Config module** (`src/config.rs`) — TOML loading, `UserCredentials`, `is_exempt()` predicate
- **API client** (`src/api.rs`) — Full Nextcloud WebDAV/tag API:
  - `create_tag`, `apply_tag` (idempotent, tolerates 409), `delete_tag` (idempotent, tolerates 404)
  - `get_tagged_files` (REPORT query), `get_file_id` (PROPFIND)
  - `ensure_tag` (find-or-create), `find_tag_by_name` (PROPFIND with display-name)
  - Serde structs for WebDAV XML deserialization
- **Startup reconciler** (`src/reconciler.rs`) — Walks local sync dir, compares against server tags, applies/removes tags to match policy. Tested against real Nextcloud.
- **Filesystem watcher** (`src/watcher.rs`) — `notify`-based recursive watcher, sends events via `tokio::sync::mpsc` channel
- **Main entrypoint** (`src/main.rs`) — Loads config, ensures tag, runs reconciler, starts watcher event loop

### Next Steps
- **Policy engine** (`src/policy.rs`) — Handle watcher events (`Create(File)`, `Modify(Name)`) with tag/untag logic
- **`FilePolicyBackend` trait** — Abstract platform-specific ACL operations (Windows `SetFileAttributesW`)
- **Local state store** (`sled`) — Persistent `path → (fileId, tag_state)` mapping to avoid redundant API calls
- **Error handling** — Replace `Box<dyn Error>` with a proper error enum
- **Logging** — Add structured logging (`tracing` crate)

## Testing with Docker

Spin up a local Nextcloud instance for testing:

```sh
sudo docker run -d -p 8080:80 -e NEXTCLOUD_ADMIN_USER=admin -e NEXTCLOUD_ADMIN_PASSWORD=test123 --name nextcloud nextcloud
```

Complete the install by visiting `http://localhost:8080` (SQLite is fine for testing). Once installed, the daemon can be tested with `cargo run` using `config/test_config.toml`.

```sh
sudo docker stop nextcloud   # pause
sudo docker start nextcloud  # resume
sudo docker rm -f nextcloud  # destroy (deletes all data)
```
