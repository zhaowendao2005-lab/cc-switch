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
| macOS | universal (`lipo` of aarch64 + x86_64), DMG + ZIP + updater `.app.tar.gz` |

No Linux builds.

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
