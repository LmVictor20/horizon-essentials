# horizon-essentials

EssentialsX for [Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft: Java Edition
26.3: homes, warps, teleport requests, kits, economy, mail, nicknames, moderation, signs, chat formats and the
rest of EssentialsX's 163 commands, on a Horizon dedicated server that **unmodified vanilla clients** join.

It is a Horizon *native mod*: [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) adds its Rust code
to a pinned Horizon Nightly at build time, and you run the resulting server binary. Status per command and
feature: [PARITY.md](PARITY.md). Contributors: [AGENTS.md](AGENTS.md).

## Build

Requirements: Linux, Rust (rustup), Python 3.10+, Git, and these checkouts beside this repository:
`horizon` (with its `nightly-source/*` tags) and `horizon-mixins` at the revision in `compatibility.json`.

```sh
python3 tools/horizon.py build      # -> target/horizon-cargo/fast/horizon-client: the server, with Essentials inside
```

## Run

Use the binary as Horizon's `horizon-engine`, with your prepared game files:

```sh
./horizon-client --resources resources --dedicated --nogui --port 25565
```

Essentials' data lives in `essentials/` in the server folder, laid out like EssentialsX's `plugins/Essentials/`
(`config.yml`, `userdata/`, `warps/`, `kits.yml`, `worth.yml`, `motd.txt`...), so an existing EssentialsX folder
copies straight in. Permissions come from `essentials/permissions.yml` (groups; no permissions plugin needed).

## Licence

GPL-3.0, as EssentialsX, from which it is ported (see NOTICE). Not affiliated with EssentialsX, Horizon,
Mojang or Microsoft.
