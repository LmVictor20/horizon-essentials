# horizon-essentials

**EssentialsX for [Horizon](https://github.com/horizon-rust/horizon)**, the Rust rewrite of Minecraft: Java
Edition 26.3.

- ✅ **Works with the normal Minecraft client.** Players join with vanilla Minecraft 26.3 and need nothing
  installed. Horizon's own client works too.
- ✅ **All 163 EssentialsX commands**, with the same commands, permissions, messages and config files.
- ✅ **No Java.** It's a single native server program: Horizon with Essentials built in.

## What you get

| | |
|---|---|
| **Teleporting** | `/home`, `/sethome`, `/warp`, `/spawn`, `/tpa`, `/tpaccept`, `/back`, `/tpr`, `/top`... |
| **Chat** | `/msg`, `/r`, `/mail`, `/nick`, `/ignore`, `/afk`, chat formats and prefixes, local chat |
| **Items** | `/kit`, `/item`, `/give`, `/repair`, `/enchant`, `/hat`, `/workbench`, `/ec`, powertools |
| **Economy** | `/balance`, `/pay`, `/baltop`, `/sell`, `/worth`, and all the Essentials signs |
| **Moderation** | `/ban`, `/tempban`, `/kick`, `/mute`, `/jail`, `/vanish`, `/invsee`, `/seen`, `/whois` |
| **Player & world** | `/fly`, `/god`, `/heal`, `/feed`, `/gamemode`, `/speed`, `/time`, `/weather` |
| **Extras** | Protect and AntiBuild rules, Discord bridge, GeoIP, XMPP |

**How complete is it?** Every command runs. 142 of the 163 work exactly like EssentialsX; 21 have small gaps,
mostly where Horizon itself can't do something yet (for example, vanished players are invisible but still
there). The full list, with the reason for each gap, is in [PARITY.md](PARITY.md).

## Install

Download `horizon-engine-essentials` from the [latest release](../../releases/latest). It runs on any 64-bit
Linux server (Debian 12, Ubuntu 22.04 or newer).

You also need Horizon's **game files**, prepared from your own copy of Minecraft with the
[Horizon launcher](https://playhorizon.dev). Nobody can ship them for you.

### Option A: Pterodactyl panel (easiest)

1. Set up a normal Horizon server with **[horizon-egg](https://github.com/LmVictor20/horizon-egg)**. Its guide
   walks you through importing the egg and uploading the game files.
2. Upload `horizon-engine-essentials` to the server's main folder and give it permission `755`.
3. In the server's **Startup** tab, change the command to:
   ```
   ./horizon-engine-essentials --resources {{RESOURCES_DIR}} --dedicated --nogui --port {{SERVER_PORT}}
   ```
4. Start the server. Type `ping` in the console: it answers `Pong!`.

Keep the file name `horizon-engine-essentials`: reinstalling the egg replaces `horizon-engine`, never this file.

### Option B: any Linux server

In an empty folder, put `horizon-engine-essentials`, your game files in `resources/`, and a file `eula.txt`
containing `eula=true` (you accept the [Minecraft EULA](https://aka.ms/MinecraftEULA)). Then:

```sh
chmod +x horizon-engine-essentials
./horizon-engine-essentials --resources resources --dedicated --nogui --port 25565
```

### First steps on the new server

Minecraft 26.3 turns the whitelist on by default. In the server console:

```
whitelist add YourName
op YourName
```

Then join with Minecraft 26.3 and try `/sethome`, `/home`, `/spawn` and `/kit tools`.

## Configure

Everything is in the `essentials/` folder, created on the first start. It uses the same layout as EssentialsX's
`plugins/Essentials/`.

| File | What it's for |
|---|---|
| `config.yml` | All of EssentialsX's settings |
| `permissions.yml` | Groups and permissions (this replaces LuckPerms) |
| `kits.yml` | Kits |
| `worth.yml` | Sell prices |
| `motd.txt`, `rules.txt`, `info.txt` | Texts shown by `/motd`, `/rules`, `/info` |
| `userdata/`, `warps/`, `spawn.yml`, `jail.yml` | Saved player data, warps, spawns, jails |

After editing a file, type `essentials reload` in the console.

### Permissions

- Players start in the `default` group: homes, tpa, msg, mail, spawn, warps, balance, pay, the tools kit...
- Server operators can use everything.
- To give a player more, add them to `permissions.yml`:

```yaml
users:
  Steve:                      # a player name or UUID
    groups: [admin]
    permissions:
    - essentials.fly          # allow
    - -essentials.home.others # deny (starts with -)
```

Permission names are EssentialsX's: see its [permissions](https://essentialsx.net/permissions) and
[commands](https://essentialsx.net/commands) lists.

### Moving from EssentialsX on Paper or Spigot

Copy the contents of your old `plugins/Essentials/` folder into `essentials/`. Homes, warps, kits, balances,
spawns and jails carry over.

### Discord, GeoIP and XMPP

These are off by default. Their settings are in `essentials/discord/`, `essentials/discordlink/`,
`essentials/geoip/` and `essentials/xmpp/`.

- **Discord:** chat both ways, join/leave/death messages, account linking. No slash commands.
- **GeoIP:** put a MaxMind `.mmdb` database file in `essentials/geoip/`.
- **XMPP:** works with XMPP servers that allow unencrypted connections.

## Updating

Download the new release, replace `horizon-engine-essentials` with it, and restart. Your settings and player
data stay.

Each release is built for one Horizon version (see `BUILD-INFO.txt` in the release). If the server can't load
your game files, prepare them again with an up-to-date Horizon launcher.

## Contributing

Issues and pull requests are welcome! Report bugs, missing behaviour or ideas in
[Issues](../../issues).

To work on the code, see [AGENTS.md](AGENTS.md): how it's organised, how a feature is built, and how the tests
work. Building it yourself:

```sh
# next to this repository: clones of horizon-rust/horizon and horizon-rust/horizon-mixins
cargo fetch --locked --manifest-path ../horizon-mixins/Cargo.toml
python3 tools/horizon.py stage            # apply the mod to the pinned Horizon version
cargo fetch --locked --manifest-path target/mixins/modded/Cargo.toml
python3 tools/horizon.py build            # -> target/horizon-cargo/fast/horizon-client
```

Tests:

```sh
cargo test --workspace         # about 390 tests, a few seconds
python3 tools/horizon.py test  # the same tests inside Horizon
python3 tools/e2e/run.py       # a real server with test players
```

Every push is built and tested by GitHub Actions, and pushing a `v*` tag publishes a release.

## Licence

GPL-3.0, like EssentialsX, which this is ported from (see [NOTICE](NOTICE)). Not affiliated with EssentialsX,
Horizon, Mojang or Microsoft.
