# Identity Rules

## Values

| Item | Value |
|---|---|
| `productName` | `CC Switch Mod` |
| `identifier` | `lab.zhaowendao.ccswitch` |
| Deep-link scheme | `ccswitchmod` |
| Legacy scheme accepted by parser | `ccswitch` |
| Icons | official icons, unchanged |

## Files that define identity

| File | What |
|---|---|
| `src-tauri/tauri.conf.json` | `productName`, `identifier`, deep-link scheme, updater endpoints/pubkey |
| `src-tauri/Info.plist` | macOS URL scheme registration |
| `src-tauri/src/deeplink/mod.rs` | `DEEPLINK_SCHEME`, `LEGACY_DEEPLINK_SCHEME`, `is_deeplink_url` |
| `src-tauri/src/deeplink/parser.rs` | accepts both schemes |
| `src-tauri/src/lib.rs` | deep-link handler, tray tooltip, log banner |
| `src-tauri/src/auto_launch.rs` | login item name (must match `productName`) |
| `src/components/settings/AboutSection.tsx` | in-app product name and links |
| `src/components/DatabaseUpgrade.tsx` | releases URL |

## Rules

- Do not rename the Cargo package or binary (`cc-switch`). Changing it cascades into WiX, Flatpak, and release asset paths.
- Do not touch `src-tauri/icons/*`.
- Only `ccswitchmod://` is registered with the OS. The parser still accepts `ccswitch://` so existing external links keep working.
- If `productName` changes, MSI becomes a separate product (side-by-side install). Document it.
- If `identifier` changes, Tauri's Store data (`app_paths.json`), window state, and single-instance socket move. Document the reset.
- Updater endpoints must point only at this fork. Never leave `dl.ccswitch.io` or the upstream GitHub endpoint.

## Checklist

- [ ] `tauri.conf.json`: productName, identifier, schemes, endpoints, pubkey
- [ ] `Info.plist`: URL name and scheme
- [ ] `deeplink` constants and parser
- [ ] `auto_launch.rs` app name
- [ ] About/update links point at this fork
- [ ] Icons untouched
