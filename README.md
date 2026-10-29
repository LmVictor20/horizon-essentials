# horizon-essentials

EssentialsX for [Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft: Java Edition
26.3: homes, warps, teleport requests, kits, an economy with signs, mail, nicknames, chat formats, bans, jails,
vanish, AFK, god, fly and the rest of EssentialsX's 163 commands, on a Horizon dedicated server that
**unmodified vanilla 26.3 clients** join.

It is a Horizon *native mod*: [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) adds its Rust code
to a pinned Horizon Nightly at build time, and the result is one server binary, `horizon-engine-essentials`.
There is no plugin folder: Essentials is compiled in.

**Status:** every command runs; 142 of 163 are complete and 21 work with documented gaps, mostly where Horizon has
no hook yet (block placement, the attacker in damage events, vanilla commands reaching command listeners) or no
support at all (per-viewer vanish, coloured item names, custom potion effects). The table per command and feature,
with each gap's reason, is [PARITY.md](PARITY.md). Not yet tried with a real vanilla client or on a live server:
back up your world first.

## Download

- **Releases:** each `v*` tag has `horizon-engine-essentials`, `SHA256SUMS` and `BUILD-INFO.txt` (the pinned
  Horizon Nightly) attached.
- **Every commit:** the *Build server* workflow uploads the same files as the `horizon-engine-essentials`
  artifact (Actions tab → a run → Artifacts).

The binary is built on Debian bookworm (glibc 2.36): it runs on Pterodactyl's `ghcr.io/parkervcp/yolks:debian`,
Debian 12+, Ubuntu 22.04+ and newer distributions, x86-64 only.

## Run it on Pterodactyl

Start from a working Horizon server (the Horizon egg: `horizon-engine` installed, your prepared game files in
`resources/`, `eula.txt` accepted). Then:

1. **Back up** the server (Backups tab), or test on a copy of the world.
2. **Upload** `horizon-engine-essentials` to the server's root folder with the file manager or SFTP, and make it
   executable (file manager → the file → Permissions → `755`). Keep the name: a Reinstall replaces
   `horizon-engine`, never this file.
3. **Startup tab:** change the startup command to
   ```
   ./horizon-engine-essentials --resources {{RESOURCES_DIR}} --dedicated --nogui --port {{SERVER_PORT}}
   ```
   (Going back to plain Horizon is the same command with `./horizon-engine`.)
4. **Start** the server. The console shows Horizon's usual startup; Essentials creates `essentials/` on the first
   start and answers `/ping` with `Pong!`.

**Game files:** the build pins one Horizon Nightly (see `BUILD-INFO.txt`). Game files prepared by the Horizon
launcher of the same period work; if a much newer launcher's files fail to load, use files prepared for that
Nightly.

**Updating:** download the newer build, upload it over `horizon-engine-essentials`, restart. Updating the egg
(Reinstall) only updates plain Horizon's `horizon-engine`.

## Configure

Everything lives in `essentials/` in the server folder, laid out like EssentialsX's `plugins/Essentials/`:

| File | What |
|---|---|
| `config.yml` | EssentialsX's config (all its options: teleport delays, homes per group, chat formats, AFK, economy...) |
| `permissions.yml` | Groups and permission nodes (EssentialsX expects LuckPerms; this file plays that part) |
| `kits.yml`, `worth.yml`, `tpr.yml` | Kits, sell prices, random-teleport area |
| `motd.txt`, `rules.txt`, `info.txt`, `book.txt`, `custom.txt` | Text files, with EssentialsX's chapters and keywords |
| `userdata/<uuid>.yml`, `warps/`, `spawn.yml`, `jail.yml` | Saved data |
| `discord/config.yml`, `discordlink/config.yml`, `xmpp/config.yml`, `geoip/` | Optional modules, off by default |

After editing, run `/essentials reload` (or restart).

**Permissions:** players without a group get the `default` group, which holds EssentialsX's regular-player set
(homes, tpa, msg, mail, spawn, warp, balance, pay, kits.tools...). Vanilla operators also get every node EssentialsX
gives ops by default. Give someone a group:
```yaml
users:
  Steve:            # a name or a UUID
    groups: [admin]
    permissions: [essentials.fly, -essentials.home.others]   # extra nodes; `-` denies
```
Node names are EssentialsX's: [permissions](https://essentialsx.net/permissions), [commands](https://essentialsx.net/commands).

**Coming from EssentialsX on Paper/Spigot:** copy the contents of `plugins/Essentials/` into `essentials/`
(config, userdata, warps, kits, worth, spawn and jails keep their formats). Bukkit world names (`world`,
`world_nether`, `world_the_end`) are read as Horizon's dimensions.

**Discord, GeoIP, XMPP:** Discord works over its REST API (chat both ways, join/quit/death, message commands,
account linking); there are no slash commands or presence. GeoIP needs you to place a MaxMind `.mmdb` file in
`essentials/geoip/`. XMPP works with servers that accept plain connections.

## Build it yourself

Pushing to GitHub builds it (see `.github/workflows/build.yml`). Locally, on Linux with rustup and Python 3, with
these checkouts beside this repository:

```
horizon-mods/
├── horizon-essentials/   this repository
├── horizon/              https://github.com/horizon-rust/horizon (must contain the pinned commit)
└── horizon-mixins/       https://github.com/horizon-rust/horizon-mixins at the revision in compatibility.json
```

```sh
cargo fetch --locked --manifest-path ../horizon-mixins/Cargo.toml
python3 tools/horizon.py stage                                   # export the pinned Horizon, apply the mod
cargo fetch --locked --manifest-path target/mixins/modded/Cargo.toml
python3 tools/horizon.py build                                   # -> target/horizon-cargo/fast/horizon-client
```
The first build takes about 8 minutes and 3-5 GB in `target/`. A binary built on a distribution newer than the
server's may not start there (glibc); the workflow builds on Debian bookworm for that reason.

**Tests:**
```sh
cargo test --workspace                        # unit tests (about 390), seconds, no Horizon needed
python3 tools/horizon.py test                 # the same tests compiled inside Horizon's game crate
python3 tools/e2e/run.py                      # end to end: a real server and scripted vanilla-protocol players
```
The end-to-end tests need the Horizon launcher's prepared game files (default:
`~/.local/share/horizon-launcher/server-runtime/game-files/horizon-client/26.3`, or `--resources`) and
`cargo build --manifest-path tools/ess-bot/Cargo.toml --target-dir target/ess-bot` after a stage.

**Moving to a newer Horizon Nightly:** set `horizon_revision` and `horizon_nightly_tag` in `compatibility.json` and
`[compatibility] horizon_revision` in every `mixins/*/horizon-mixin.toml`, update the `source_sha256` entries,
then stage, test and build. A hook whose target changed fails at stage time and names it.

Contributors: [AGENTS.md](AGENTS.md) describes the layout, the test kit and how a feature is built.

## Licence

GPL-3.0, as EssentialsX, from which it is ported (see [NOTICE](NOTICE)). Not affiliated with EssentialsX,
Horizon, Mojang or Microsoft.
