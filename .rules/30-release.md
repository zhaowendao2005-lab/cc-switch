# Release Rules

## Authority

- Releases are built only by `.github/workflows/release-mod.yml`.
- Upstream `.github/workflows/release.yml` is kept for reference but is not used. Do not push `v*` tags.
- Tag format: `mod-v<upstream-version>`, e.g. `mod-v4.0.28`.
- App version = upstream version. One release per upstream stable version; for a mod-only hotfix bump the patch and document it.

## Platforms

| Platform | Target |
|---|---|
| Windows x64 | `windows-2022`, MSI + portable zip |
| Windows ARM64 | `windows-11-arm`, MSI |
| macOS | `macos-15-intel`, `macos-compile` matrix builds aarch64 + x86_64 in parallel, `macos-release` `lipo`s them into universal + DMG/ZIP/updater `.app.tar.gz` |

No Linux builds.

Do not switch macOS back to `macos-15`/`macos-latest`: those are arm64 runners whose
free pool regularly cancels queued jobs after the queue timeout. The Intel runner is
available and can cross-compile `aarch64-apple-darwin`, so both matrix jobs run on
`macos-15-intel` and each builds a single architecture.

`macos-compile` only produces a binary (uploaded as a short-lived artifact);
`macos-release` downloads both, merges them with `lipo`, and does the bundling,
ad-hoc signing and asset packaging. Keep the packaging job free of Rust compilation
so a cache miss or slow compile cannot delay it.

## Build caching

- Rust dependencies are cached with `Swatinem/rust-cache`, one cache per target
  (`key: release-<target>`); the action keys on OS/arch, rustc version and the
  dependency set in `Cargo.lock`.
- Caches are saved only on publish runs (tag push or `publish=true`); dry-runs restore
  read-only so a test run cannot evict the real caches.
- The pnpm store is cached through `actions/setup-node` (`cache: pnpm`); this is why
  `pnpm/action-setup` must run before `setup-node`.
- GitHub gives a repository 10 GB of cache in total, shared with CI. Treat a cache miss
  as a slower build only: never make the workflow depend on a cache hit.

## macOS signing

- No Apple Developer account.
- Build with `APPLE_SIGNING_IDENTITY="-"` (ad-hoc).
- Do not import a certificate, notarize, staple, or run `spctl`.
- Keep `codesign --verify --deep --strict`.
- Document Gatekeeper: right-click Open, or `xattr -dr com.apple.quarantine "/Applications/CC Switch Mod.app"`.

## Updater keys

- Public key is committed in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).
- Private key and password live only in GitHub Secrets:
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- Signing happens in Actions via `.github/actions/prepare-tauri-signing-key`.
- Do not commit private keys, `.p12`, `.pem`, or passwords.
- Rotating the public key requires shipping a build signed with the old key or asking users to reinstall.

## Assets and manifest

- `latest.json` must include only:
  - `windows-x86_64`
  - `windows-aarch64`
  - `darwin-aarch64`
  - `darwin-x86_64`
- Required signatures: Windows MSIs + macOS `.app.tar.gz`. Fail the job if any are missing.
- Publish with `prerelease: false` and `make_latest: true` so `/releases/latest/download/latest.json` resolves.

## Version files

All must match the tag:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.lock` (`cc-switch` entry)
- `src/whats-new/<version>.json`

Use `scripts/bump-mod-version.mjs`; the release workflow only verifies.

## Secrets / environment

- Set `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Create a `release` environment for approval if desired.
- Do not print key material in workflow logs.
