# Upstream Sync Rules

## Workflow

`.github/workflows/sync-upstream.yml` runs every 30 minutes and follows the latest **stable** release of `farion1231/cc-switch` (via `/releases/latest`, which excludes prereleases and drafts).

## Flow

1. Resolve the latest stable tag.
2. Skip if `sync/<tag>` already exists.
3. Fetch the upstream tag into `refs/tags/upstream-<tag>` and merge into `main`.
4. On conflict: abort, leave `main` unchanged, open an issue, fail the job. Never use `-X ours`.
5. On success: push `main`, push `sync/<tag>`.
6. Run `scripts/bump-mod-version.mjs <tag>`; commit `chore(release): <tag>`.
7. Create/push `mod-v<version>`.
8. Dispatch `release-mod.yml` with `tag=mod-v<version>` and `publish=true`.

## Rules

- `GITHUB_TOKEN` cannot trigger `push` workflows, but `workflow_dispatch` is an explicit exception; the workflow needs `actions: write`.
- Guard the release dispatch on the same condition that performed the merge, so no-op runs never release.
- Dedupe with `sync/<tag>`, `gh release view`, and concurrency groups.
- If organization policy blocks token dispatch, use a fine-grained PAT (`SYNC_PAT`) only for the dispatch step.
- Do not fetch/push upstream `v*` tags.
- Conflicts must be resolved manually; reapply the identity and DB changes listed in `.rules/10-identity.md` and `.rules/20-database.md`.

## Conflict hotspots

- `src-tauri/tauri.conf.json`
- `src-tauri/Info.plist`
- `src-tauri/src/lib.rs` (startup + `generate_handler!`)
- `src-tauri/src/database/mod.rs`, `backup.rs`
- `src/components/settings/AboutSection.tsx`
- `.github/workflows/sync-upstream.yml`
