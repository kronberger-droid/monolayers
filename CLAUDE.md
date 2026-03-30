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
