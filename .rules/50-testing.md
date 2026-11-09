# Testing Rules

## Local checks (must pass)

```bash
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test:unit
pnpm format:check
pnpm build:renderer

cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Use `CC_SWITCH_TEST_HOME="$(mktemp -d)"` when running backend tests that touch user data.

## Database fallback simulation

Use an isolated home or a copied `~/.cc-switch`:

1. **Primary compatible**: start app; primary is active and `mod_cc-switch.db` is mirrored.
2. **Primary too new**: set `PRAGMA user_version = 999`; app selects fallback; primary file unchanged.
3. **Primary corrupt**: truncate/corrupt primary; app selects fallback.
4. **Primary older**: set an older `user_version`; app copies + migrates fallback, primary unchanged.
5. **Both unusable**: recovery UI appears; confirm no DDL ran on either.
6. **Dirty fallback**: run in fallback mode, make a change; after restart auto-sync is skipped and status shows dirty.
7. **Force sync**: creates `mod_db_backup_*.db`, then overwrites fallback.
8. **Backups**: primary lists `db_backup_*`; fallback lists `mod_db_backup_*`.

## Release checks

- `workflow_dispatch` `release-mod.yml` with `publish=false`.
- Verify Windows x64 + ARM64 MSI and `.sig`, macOS `.app.tar.gz` + `.sig`, DMG/ZIP.
- Verify `latest.json` has exactly the four platform keys.
- Verify the built binary contains the committed public key, not the placeholder.
- After publishing: `/releases/latest/download/latest.json` resolves and a previous build updates.

## Updater

- macOS: ad-hoc signed, not notarized. Confirm it launches; document Gatekeeper workaround.
- Windows: unsigned MSI; SmartScreen warning expected.
