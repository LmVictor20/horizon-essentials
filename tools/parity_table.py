#!/usr/bin/env python3
"""Writes PARITY.md's skeleton from mixins/essentials/ess_meta.rs (every EssentialsX command, by area).

  python3 tools/parity_table.py > PARITY.md      (only to start it: afterwards the team edits PARITY.md)
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
AREAS = {
    "tp": "back bottom delhome home jump renamehome sethome setspawn spawn settpr setwarp delwarp warp warpinfo top tp tpa "
          "tpaall tpacancel tpaccept tpahere tpall tpauto tpdeny tphere tpo tpoffline tpohere tppos tpr tptoggle world",
    "social": "afk broadcast broadcastworld customtext help helpop ignore info mail me motd msg msgtoggle mute nick r realname "
              "rtoggle rules socialspy toggleshout",
    "admin": "backup ban banip compass deljail depth essentials gc getpos invsee jailedplayers jails kick kickall kill list near "
             "playtime seen setjail sudo tempban tempbanip togglejail unban unbanip vanish whois",
    "items": "anvil book cartographytable clearinventory clearinventoryconfirmtoggle condense createkit delkit disposal enchant "
             "enderchest exp firework give grindstone hat item itemdb itemlore itemname kit kitreset loom more potion powertool "
             "powertoollist powertooltoggle recipe repair showkit skull smithingtable stonecutter unlimited workbench",
    "economy": "balance balancetop eco pay payconfirmtoggle paytoggle sell setworth worth",
    "world": "antioch beezooka bigtree break burn editsign ext feed fireball fly gamemode god heal ice kittycannon lightning nuke "
             "ptime pweather remove rest speed spawner spawnmob suicide thunder time tree weather discord discordbroadcast link "
             "unlink setxmpp xmpp xmppspy",
    "core": "ping",
}
FEATURES = {
    "tp": ["teleport-delay warmup, cancelled by moving or damage (essentials.teleport.timer.*)", "teleport-cooldown and its bypasses",
           "teleport-safety / force-safe-teleport-location / is-water-safe", "teleport-invulnerability", "teleport-to-center",
           "/back on teleport and on death (back.ondeath, back.into.<world>)", "tpa-accept-cancellation, tpa-max-requests",
           "sethome-multiple groups and limits, confirm-home-overwrite, world-home-permissions", "per-warp-permission, warp list paging",
           "Spawn module: spawn-on-join, newbies (spawnpoint, kit, announce-format), respawn-at-home(-bed), respawn-at-anchor",
           "random-spawn-location, random-respawn-location, tpr.yml areas and excluded biomes", "world-teleport-permissions"],
    "social": ["Chat module: format, group-formats, placeholders, world-aliases", "local chat radius, ! shout, ? question, chat spy (chat.spy)",
               "chat colour/format/magic/rgb/url permissions", "custom-join-message, custom-quit-message, custom-new-username-message, silent join/quit",
               "motd on join (delay-motd)", "mail: notify on join and every notify-player-of-mail-cooldown, mails-per-minute, sendtemp expiry",
               "AFK: auto-afk, auto-afk-timeout kick/commands, freeze, cancel-on-*, broadcast, afk-list-name, sleep-ignores-afk",
               "ignore: chat, msg, mail, tpa filtered", "socialspy messages and socialspy-commands", "mute: chat, msg, mail, mute-commands",
               "nicknames: prefix, length, regex, blacklist, uniqueness, ops-name-color, change-displayname/tab", "text files: chapters, pages, keywords",
               "message overrides: messages*.properties in the data folder, per-player-locale"],
    "admin": ["permissions provider: permissions.yml groups, inheritance, wildcards, negation, prefix/suffix", "ban/tempban/ipban enforced at login",
              "jail: restrictions (break/place/interact/commands), jail-online-time, teleport-when-freed", "vanish: tab list, entity, /list, status sample",
              "invsee: live view and edits", "disabled-commands, overridden-commands, command-cooldowns (+persistence)", "log-console-commands",
              "kickall/kick exempt nodes, ban notify", "seen: alts by IP, IP search"],
    "items": ["kits.yml format (items, meta, commands), delays, one-time kits, kit-auto-equip, drop-items-if-full", "items.json item aliases (itemdb)",
              "unlimited items (no consumption on place)", "powertools: commands on interact ({player})", "item-spawn-blacklist, permission-based-item-spawn",
              "oversized stacks", "virtual containers: disposal, kit preview, enderchest of others", "portable workstations (no block)"],
    "economy": ["balances, starting-balance, min-money / max-money / eco.loan", "currency-symbol (+suffix) formatting", "economy-log (CSV)",
                "worth.yml, sell-multipliers, allow-bulk-buy-sell, allow-selling-named-items", "baltop cache, baltop-requirements, entry limit",
                "pay confirmation, minimum-pay-amount, pay-excludes-ignore-list", "command-costs (incl. chat-shout/question), nocommandcost",
                "signs: [Balance] [Buy] [Sell] [Trade] [Free] [Disposal] [Enchant] [Repair] [Heal] [Gamemode] [Info] [Kit] [Mail] "
                "[RandomTeleport] [Warp] [Anvil] [Cartography] [Grindstone] [Loom] [Smithing] [Workbench] [Spawnmob] [Time] [Weather] "
                "[Protection], colour signs, sign-use-per-second, enabledSigns"],
    "world": ["god mode (no damage, no hunger)", "fly: persistence, safelogin, world-change reset", "speed caps (max-walk-speed, max-fly-speed)",
              "death: vanishing/binding items policy, keepinv, keepxp", "Protect module: fire, lava, explosions, mob spawns, damage types",
              "AntiBuild module: build/break/use/drop/pickup permissions, blacklists, alerts", "GeoIP: database, join message, whois line",
              "Discord: chat both ways, events, console relay, commands", "DiscordLink: link codes, role sync, unlinked kick", "XMPP bridge"],
}

meta = (ROOT / "mixins/essentials/ess_meta.rs").read_text()
commands = re.findall(r'CommandMeta \{ name: "([a-z]+)", module: "([a-z]+)", aliases: &\[([^\]]*)\]', meta)
area_of = {c: a for a, names in AREAS.items() for c in names.split()}
missing = [c for c, _, _ in commands if c not in area_of]
assert not missing, f"commands without an area: {missing}"

print("# Parity with EssentialsX\n")
print("The definition of done: every EssentialsX command and feature works on Horizon with unmodified vanilla 26.3 clients,")
print("or is marked `partial` with the reason and a test pinning what we deliver. Status: `todo`, `tests` (failing tests")
print("written), `done` (L1 green, glue in, L3 green where it has a scenario), `partial` (reason in Notes).\n")
print("Generated once by tools/parity_table.py from plugin.yml; each area keeps its own rows current.\n")
for area in ["core", "tp", "social", "admin", "items", "economy", "world"]:
    rows = [(c, m, a) for c, m, a in commands if area_of[c] == area]
    print(f"## {area} ({len(rows)} commands)\n")
    print("| Command | Module | Aliases | Status | Notes |")
    print("|---|---|---|---|---|")
    for c, module, aliases in rows:
        names = [x.strip().strip('"') for x in aliases.split(",") if x.strip()]
        shown = ", ".join(n for n in names if not (n.startswith("e") and n[1:] in names + [c]))
        status = "done" if c == "ping" else "todo"
        print(f"| /{c} | {module} | {shown} | {status} | |")
    if area in FEATURES:
        print(f"\n**Features ({area})**\n")
        print("| Feature | Status | Notes |")
        print("|---|---|---|")
        for f in FEATURES[area]:
            print(f"| {f} | todo | |")
    print()
print(f"Total: {len(commands)} commands.")
