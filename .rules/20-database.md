# Database Rules

## Files

- Primary DB: `~/.cc-switch/cc-switch.db`
- Fallback DB: `~/.cc-switch/mod_cc-switch.db`
- Sync state: `~/.cc-switch/mod-db-state.json`
- Code: `src-tauri/src/database/{selector.rs,fallback_sync.rs,backup.rs,mod.rs}`

## Selection rule: exact match only

Compatibility means **exactly** `PRAGMA user_version == SCHEMA_VERSION`.

| Primary state | Active DB |
|---|---|
| missing, fallback absent | primary (create fresh) |
| missing, fallback compatible | fallback |
| `user_version == SCHEMA_VERSION` | primary |
| `user_version < SCHEMA_VERSION` | fallback; if fallback missing, copy primary to fallback and migrate the copy |
| `user_version > SCHEMA_VERSION` | fallback; never open primary writable |
| unreadable / corrupt | fallback |
| both unusable | recovery UI |

**Never migrate the primary in place when its version differs.** An installed official build may still be using it.

`probe_database` opens read-only and must not run DDL, `create_tables`, migrations, or `auto_vacuum`. Zero-byte files count as compatible (tests use them as sentinels).

## One-way sync: primary → fallback

- Only when primary is active and exact-compatible.
- Snapshot via the SQLite Backup API into a temp file, `PRAGMA quick_check`, then atomically replace `mod_cc-switch.db`.
- Never sync fallback → primary.
- When fallback is selected, set `fallbackDirty: true` before opening it. Auto-sync stays disabled until a human runs a force sync.
- Force sync first writes `mod_db_backup_<timestamp>.db` (in `backups/`) before overwriting.
- Throttle automatic sync (about 10 minutes). Run once after startup and periodically; never block the window.

## Schema rules for mods

- **Never change `SCHEMA_VERSION`.**
- Do not add columns to upstream tables. Use a separate table prefixed `mod_` and key by the upstream row ID.
- Create mod tables idempotently in whichever DB is active; they must work in both primary and fallback.
- Mod data written while fallback is active is not merged back to primary by design. Document this for users.
- `settings.json` remains device-global at `~/.cc-switch/settings.json`; it does not switch with the DB.

## Backups

- Primary backups: `db_backup_*.db`
- Fallback backups: `mod_db_backup_*.db`
- All backup operations must use `Database.path`, not a hardcoded `cc-switch.db`.
- Retention must not evict primary and fallback backups against each other.

## Tests

- selector: missing / zero-byte / exact / older / newer / corrupt / both unusable
- `init_with_path` rejects too-new DB **before** any DDL
- copy-and-migrate older primary
- sync copies rows; skipped when dirty; force sync creates a safety backup
- path-aware backup listing and prefix filtering
