# AGENTS

This repository is a self-maintained mod of [farion1231/cc-switch](https://github.com/farion1231/cc-switch).

**Read `.rules/README.md` before changing identity, database, release, or upstream-sync code.**

Non-negotiables:

- Do not change `src-tauri/icons/*`.
- Never write or migrate a primary DB whose `user_version` is newer than `SCHEMA_VERSION`.
- The fallback DB is `~/.cc-switch/mod_cc-switch.db`; never use a separate config directory.
- Never change `SCHEMA_VERSION` for a mod. Mod tables must be additive and prefixed `mod_`.
- Release only through `.github/workflows/release-mod.yml`; tags are `mod-v*`.
- Never push upstream `v*` tags to this fork.
- Private updater key material lives only in GitHub Actions Secrets.
- Apply existing identity values (`CC Switch Mod`, `lab.zhaowendao.ccswitch`, `ccswitchmod`) consistently.
