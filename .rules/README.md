# Mod Development Rules

This repository is a self-maintained mod of [farion1231/cc-switch](https://github.com/farion1231/cc-switch). It follows upstream stable releases automatically and adds fork-only features on top.

Read these files before changing identity, database, release, or sync code:

1. [10-identity.md](10-identity.md) — product name, identifier, deep-link scheme, icons.
2. [20-database.md](20-database.md) — primary/fallback DB selection, one-way sync, schema safety.
3. [30-release.md](30-release.md) — `release-mod.yml`, unsigned macOS, updater keys, tags.
4. [40-upstream-sync.md](40-upstream-sync.md) — auto-follow workflow, conflicts, version bumps.
5. [50-testing.md](50-testing.md) — local checks, fallback simulation, release dry-run.

## Non-negotiables

- **Never change `src-tauri/icons/*`.** The fork intentionally keeps the official icons.
- **Never migrate or write a primary DB whose `user_version` is newer than `SCHEMA_VERSION`.**
- **Never change `SCHEMA_VERSION` for a mod.** Mod tables must be additive and prefixed `mod_`.
- **The fallback DB is `~/.cc-switch/mod_cc-switch.db`**, not a separate config directory.
- **Releases only come from `.github/workflows/release-mod.yml`** with tags `mod-v*`.
- **Private key material lives only in GitHub Actions Secrets.** Public updater key is committed.
- **Do not push upstream `v*` tags** to this fork.

## Mod code layout

Prefer new files over edits to upstream files:

| Layer | Location |
|---|---|
| Rust mod modules | `src-tauri/src/mods/<feature>.rs` |
| Frontend mod components | `src/mods/<feature>/` |
| DB probe/fallback | `src-tauri/src/database/selector.rs`, `fallback_sync.rs` |
| Tauri commands | `src-tauri/src/commands/` + register in `lib.rs` `generate_handler!` |
| Frontend API wrappers | `src/lib/api/` |
| Release automation | `.github/workflows/release-mod.yml`, `scripts/bump-mod-version.mjs` |

Shared registration points are the main merge-conflict hotspots:

- `src-tauri/src/lib.rs` (`generate_handler!`, startup wiring)
- frontend router / sidebar
- `src/i18n/locales/*.json`
- `src-tauri/tauri.conf.json`
- `.github/workflows/sync-upstream.yml`

Keep edits to those files small and localized.
