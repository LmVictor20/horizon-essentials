# Parity with EssentialsX

The definition of done: every EssentialsX command and feature works on Horizon with unmodified vanilla 26.3 clients,
or is marked `partial` with the reason and a test pinning what we deliver. Status: `todo`, `tests` (failing tests
written), `done` (L1 green, glue in, L3 green where it has a scenario), `partial` (reason in Notes).

Generated once by tools/parity_table.py from plugin.yml; each area keeps its own rows current.

## core (1 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /ping | core | echo, pong | done | |

## tp (32 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /back | core | return | done | L1 + L3 (tp_homes): to the last location, back.into.<world>, others, console; /back on teleport and death |
| /bottom | core |  | done | L1: LocationUtil.getSafeDestination from the bottom of the column |
| /delhome | core | remhome, rmhome | done | L1: player:name, `*`, numbered homes, console |
| /delwarp | core | remwarp, rmwarp | done | L1 |
| /home | core | homes | done | L1 + L3 (tp_homes): list with bed, numbers, player:name, bed, spawn-if-no-home, world-home-permissions |
| /jump | core | j, jumpto | partial | L1: jump to the target block (safe, centered); `/jump lock` toggles the flag but the fly-click jump itself (right click with a compass, powertool-style) is not wired (needs the items area's interact hook) |
| /renamehome | core |  | done | L1 |
| /sethome | core | createhome | done | L1 + L3 (tp_homes): limits, name forced to `home` at a limit of 1, invalid names, unsafe spots, confirm-home-overwrite, others |
| /settpr | core | settprandom | done | L1 + L3 (tp_places): center/minrange/maxrange in tpr.yml |
| /setwarp | core | createwarp | done | L1 + L3 (tp_places): warps/<sanitized>.yml with name and lastowner, overwrite node, similar names, reserved names |
| /top | core |  | done | L1 + L3 (tp_homes) |
| /tp | core | tele, teleport, tp2p | done | L1 + L3 (tp_requests): player, player to player, positions (~ relative, 30M bound), offline hint, console forms |
| /tpa | core | call, tpask | partial | L1 + L3 (tp_requests); requests from players the target ignores are not dropped yet (ignore lists are the social area's: TODO, CORE-REQUESTS #3) |
| /tpaall | core |  | done | L1 |
| /tpacancel | core |  | done | L1 |
| /tpaccept | core | tpyes | done | L1 + L3 (tp_requests): newest first, by name, `*`/all, here-requests use the stored location |
| /tpahere | core |  | partial | L1 + L3 (tp_requests); ignore lists not honoured yet (social, TODO) |
| /tpall | core |  | done | L1 |
| /tpauto | core |  | done | L1 |
| /tpdeny | core | tpno | done | L1 + L3 (tp_requests) |
| /tphere | core | s | done | L1 + L3 (tp_requests) |
| /tpo | core |  | done | L1 |
| /tpoffline | core | otp, offlinetp, tpoff, tpoffline | done | L1: to the logout location |
| /tpohere | core |  | done | L1 |
| /tppos | core |  | done | L1: yaw/pitch/world forms, console form |
| /tpr | core | tprandom | partial | L1 + L3 (tp_places); random spots in chunks that are not loaded count as failed attempts (no synchronous chunk load from a mod), so with the default max-range (half the world border) /tpr mostly lands on the center: set a max-range around loaded terrain. No location cache (cache-threshold): spots are computed on demand |
| /tptoggle | core |  | done | L1 |
| /warp | core | warps | done | L1 + L3 (tp_places): pages of 20, warp.list, per-warp-permission, others, console |
| /warpinfo | core |  | done | L1 + L3 (tp_places) |
| /world | core |  | done | L1: nether toggle with the 8:1 scale, by name or index, invalid-world help |
| /setspawn | spawn |  | done | L1 + L3 (tp_places): spawn.yml spawns.<group> |
| /spawn | spawn |  | done | L1 + L3 (tp_places): group spawn, default, world spawn; others; console |

**Features (tp)**

| Feature | Status | Notes |
|---|---|---|
| teleport-delay warmup, cancelled by moving or damage (essentials.teleport.timer.*) | done | L1: checked every second like AsyncTimedTeleport (moving about 3 blocks or losing health cancels); timer.bypass, timer.move |
| teleport-cooldown and its bypasses | done | L1: timestamps.lastteleport; cooldown.bypass, .back, .tpa by teleport type |
| teleport-safety / force-safe-teleport-location / is-water-safe | partial | L1 + L3 (/top): LocationUtil ported (hollow and damaging blocks, nether roof via consider-world-height-for-teleport-safety); a destination in a chunk that is not loaded is used as given (centered) since its blocks cannot be read; the world border is not consulted (vanilla's is never in the way) |
| teleport-invulnerability | partial | L1: Host::set_invulnerable for N s after a command teleport (all damage, not only PvP, since there is no PvP-only damage hook); the damage listener also cancels player attacks if the world area routes damage |
| teleport-to-center | done | L1 |
| /back on teleport and on death (back.ondeath, back.into.<world>) | done | L1 + L3 (tp_homes): deaths are seen by polling Guest.dead each tick (Host::is_dead) because Game::guest_die holds macros, which HEAD refuses |
| tpa-accept-cancellation, tpa-max-requests | done | L1: expiry message; at the limit the newest request is dropped (as EssentialsX) |
| sethome-multiple groups and limits, confirm-home-overwrite, world-home-permissions | done | L1 |
| per-warp-permission, warp list paging | done | L1 |
| Spawn module: spawn-on-join, newbies (spawnpoint, kit, announce-format), respawn-at-home(-bed), respawn-at-anchor | partial | L1 (+ L3 for /spawn, /setspawn): spawn-on-join groups and exempt, newbies spawnpoint and announcement, respawn at group spawn / home / bed / anchor, respawn-listener-priority and spawn-join-listener-priority `none`. newbies.kit is not given (items area: TODO, CORE-REQUESTS #4). The respawn point is applied one tick after vanilla respawns the player (a second teleport); respawn is not covered at L3 (ess-bot cannot respawn) |
| random-spawn-location, random-respawn-location, tpr.yml areas and excluded biomes | partial | L1: see /tpr (unloaded chunks count as failed attempts; no cache) |
| world-teleport-permissions | done | L1: tp, tpa, tpahere, tpaccept, tpo, tphere, tppos, tpoffline, world, back |

Area-wide notes (tp): messages name players by their account name (nicknames and prefixes are the social area's, CORE-REQUESTS #2); command costs (`command-costs`, Trade charges) are not taken (economy area).

## social (21 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /afk | core | away | done | others, messages (essentials.afk.message), mute check; L3 social_chat |
| /broadcast | core | bc, bcast, shout | done | `&` codes, `\n` lines; L3 social_chat |
| /broadcastworld | core | bcw, bcastw, shoutworld | done | world by Bukkit name (world, world_nether, world_the_end) or id |
| /customtext | core |  | partial | `/customtext <chapter> [page]` works; commands.yml aliases (`/vote` -> customtext) don't exist on Horizon |
| /help | core |  | done | help.txt, or HelpInput over Essentials' commands the player may use; `/help /<cmd>` usage |
| /helpop | core | ac, amsg | done | essentials.helpop.receive; L3 social_chat |
| /ignore | core | unignore, delignore, remignore, rmignore | done | userdata `ignore` (uuids), essentials.chat.ignoreexempt; L3 social_msg |
| /info | core | about, ifo, inform, news | done | chapters, pages, keywords, per-player/group files |
| /mail | core | memo | done | read/clear/clearall/send/sendtemp/sendall/sendtempall, console; userdata `mail` as EssentialsX; L3 social_mail |
| /me | core | action, describe | done | chat radius, ignore, spies; L3 social_chat |
| /motd | core |  | done | |
| /msg | core | w, m, t, pm, tell, whisper | done | console, wildcards (essentials.msg.multiple), colors by permission; L3 social_msg |
| /msgtoggle | core |  | done | essentials.msgtoggle.bypass |
| /mute | core | silence, unmute | done | time + reason, max-mute-time, exempt/offline/notify, expiry on the 50-tick timer |
| /nick | core | nickname | done | prefix, length, regex, blacklist, uniqueness, colors by permission; L3 social_chat |
| /r | core | reply | done | last-message-reply-recipient + timeout; L3 social_msg |
| /realname | core |  | done | |
| /rtoggle | core | replytoggle | done | userdata `last-message-reply-recipient` |
| /rules | core |  | done | |
| /socialspy | core |  | done | |
| /toggleshout | chat |  | done | chat.persist-shout / shout-default |

**Features (social)**

| Feature | Status | Notes |
|---|---|---|
| Chat module: format, group-formats, placeholders, world-aliases | partial | all placeholders but {TEAMPREFIX}/{TEAMSUFFIX}/{TEAMNAME} (empty: no scoreboard-team API); {PREFIX}/{SUFFIX}/{GROUP} from permissions.yml. Chat equal to vanilla's stays signed player chat, anything else is re-sent as system chat (L3 social_chat) |
| local chat radius, ! shout, ? question, chat spy (chat.spy) | done | |
| chat colour/format/magic/rgb/url permissions | done | per-color nodes too (essentials.chat.red...) |
| custom-join-message, custom-quit-message, custom-new-username-message, silent join/quit | partial | join/quit/silent/hide-join-quit-messages-above done (vanilla lines suppressed by the hub hooks); new-username message and {OLDUSERNAME} need the previous name (CORE-REQUESTS 3) |
| motd on join (delay-motd) | done | L3 social_chat |
| mail: notify on join and every notify-player-of-mail-cooldown, mails-per-minute, sendtemp expiry | done | L3 social_mail |
| AFK: auto-afk, auto-afk-timeout kick/commands, freeze, cancel-on-*, broadcast, afk-list-name, sleep-ignores-afk | partial | done except sleep-ignores-afk-players, disable-item-pickup-while-afk, cancel-afk-on-fish (no sleep/pickup/fishing events); cancel-on-interact only on right clicks the core reports |
| ignore: chat, msg, mail, tpa filtered | partial | chat, msg, mail, /me, broadcasts done; tpa belongs to tp (`ess_social::ignores_player`) |
| socialspy messages and socialspy-commands | partial | Essentials' commands only: vanilla/other commands don't reach command listeners (CORE-REQUESTS 2) |
| mute: chat, msg, mail, mute-commands | partial | mute-commands only for Essentials' commands (CORE-REQUESTS 2) |
| nicknames: prefix, length, regex, blacklist, uniqueness, ops-name-color, change-displayname/tab | partial | all rules done (own java.util.regex subset); the nick shows in Essentials' chat lines and the tab list (change-playerlist, afk-list-name), not in vanilla's join/leave lines (CORE-REQUESTS 5) |
| text files: chapters, pages, keywords | done | {BALANCE} reads userdata `money`; {PLUGINS}/{VERSION} are fixed strings |
| message overrides: messages*.properties in the data folder, per-player-locale | partial | overrides by the core; English only (no per-player locale) |
| command tree: every Essentials label tab-completes on vanilla clients | done | greedy-string nodes merged into the hub's mod command tree |

## admin (28 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /backup | core |  | done | save-all/save-off, the configured `backup.command` in the background, then save-on and "Backup finished."; periodic `backup.interval` and `always-run` |
| /ban | core |  | done | written to banned-players.json; offline/exempt nodes, `ban.notify`; L3 admin_bans |
| /banip | core |  | done | by address or player's last IP; kicks online players on that IP; L3 admin_bans |
| /compass | core | direction | done | |
| /deljail | core | remjail, rmjail | done | |
| /depth | core | height | done | sea level 63 (nether 32, end 0) |
| /essentials | core | ess, essversion | partial | reload (config, permissions, messages, jail.yml), version, commands, debug, cleanup, homes fix/delete, usermap, moo, nyan (note blocks via /playsound). No `dump`, `itemtest` or `reset`: they report Bukkit/plugin internals |
| /gc | core | lag, mem, memory, uptime, tps, entities | done | TPS from the server clock's average tick time (20 when ticks take under 50 ms); memory is the process RSS against the machine's RAM, since there is no JVM heap; per-dimension chunks, mobs and block entities |
| /getpos | core | coords, position, whereami, getlocation, getloc | done | Bukkit world names (`world`, `world_nether`, `world_the_end`); L3 admin_info |
| /invsee | core |  | partial | opens a 5-row copy of the inventory (or the 1-row `equip` view); `invsee.modify` / `invsee.preventmodify` decide whether clicks are allowed, and edits are written back when it closes. The view doesn't update live, and the container glue (Host::open_container) belongs to the items area |
| /jailedplayers | core | ejailed, ejp | done | every jailed user, online or not; L3 admin_jail |
| /jails | core |  | done | L3 admin_jail |
| /kick | core |  | done | `kick.exempt`, `kick.notify`, hidden targets; L3 admin_bans |
| /kickall | core |  | done | spares the sender and `kickall.exempt` |
| /kill | core |  | done | `kill.exempt` / `kill.force`, cancelled damage, `*`; sets health through the world area's glue |
| /list | core | online, playerlist, plist, who | done | guests' `/list` is rerouted from Horizon's built-in answer (guest_command HEAD hook); groups from permissions.yml, `list:` merge/hidden/limit/`*`, `[HIDDEN]`/`[AFK]` tags; L3 admin_info |
| /near | core | nearby | done | |
| /playtime | core |  | done | `minecraft:play_time` from live counters, or the stats file for offline players |
| /seen | core | alts | partial | online/offline, bans and expiry, IP, logout location, whitelist, UUID, alts and IP search (L3 admin_info). "First login" is omitted (Horizon keeps no first-join time); no GeoIP |
| /setjail | core | createjail | done | jail.yml as EssentialsX writes it (world as a dimension id, plus `world-name`); L3 admin_jail |
| /sudo | core |  | done | Essentials or vanilla commands next tick; `c:` chat goes through the chat listeners |
| /tempban | core |  | done | DateUtil durations, `max-tempban-time`, `tempban.unlimited/offline/exempt`; L3 admin_bans |
| /tempbanip | core |  | done | keeps EssentialsX's argument order (reason shown as {2}) |
| /togglejail | core | jail, tjail, unjail | done | teleport to jail, timed sentences, extensions, `jail.exempt`, offline jailing applied on join; L3 admin_jail |
| /unban | core | pardon | done | L3 admin_bans |
| /unbanip | core | pardonip | done | L3 admin_bans |
| /vanish | core | v | partial | hidden from /list, tab completion, player matching, /near and the tab list (unlisted for players without `vanish.see`); `vanish.effect` gives invisibility. The player entity stays tracked for everyone: Horizon's entity tracking is shared by all viewers. L3 admin_info |
| /whois | core |  | partial | every EssentialsX line. Speed is always the default 0.2/0.1, because Horizon keeps no Bukkit speeds; AFK "since" needs the social area's timestamp; no GeoIP |

**Features (admin)**

| Feature | Status | Notes |
|---|---|---|
| permissions provider: permissions.yml groups, inheritance, wildcards, negation, prefix/suffix | partial | tested in ess_admin (groups, default group, inheritance chains and loops, `a.*`/`*`, `-node`, users by name or UUID, plugin.yml op defaults, reload). Plugin.yml children of configured parent nodes are not expanded, and a group's `-node` beats a user's `node`: CORE-REQUESTS 1-2 |
| ban/tempban/ipban enforced at login | partial | bans go to the vanilla lists, so vanilla's login check refuses banned players (L3 admin_bans). The refusal text is vanilla's: `Lists::can_login` contains `vec!` macros that horizon-mixins can't hook. Players who still get in (integrated server) are kicked on join with banJoin/tempbanJoin/banIpJoin/tempbanIpJoin; expired tempbans are removed |
| jail: restrictions (break/place/interact/commands), jail-online-time, teleport-when-freed | partial | break, place, interact, Essentials commands (`jail.allow.<cmd>`), respawn in jail, sent back on join, timeouts checked every second, `jail-online-time` (play_time), `teleport-when-freed` back/spawn/off. Not covered: teleports out, hitting players, game-mode changes and vanilla commands, because the core has no such events (CORE-REQUESTS 3, 5) |
| vanish: tab list, entity, /list, status sample | partial | tab list, /list, `can_see`, re-hidden for players who join later, unvanished on quit, `silentjoin.vanish`. Entity hiding is global only (invisibility effect), and the status sample is not filtered |
| invsee: live view and edits | partial | see /invsee: a snapshot, edited on close |
| disabled-commands, overridden-commands, command-cooldowns (+persistence) | partial | disabled-commands (left to vanilla) and cooldowns (`*` wildcards, `^regex`, bypass nodes, persisted in userdata `timestamps.command-cooldowns` or kept in memory) apply to Essentials commands only (CORE-REQUESTS 3). overridden-commands has nothing to yield to: Horizon has no other plugins |
| log-console-commands | done | "CONSOLE issued server command: /..." in the server log |
| kickall/kick exempt nodes, ban notify | done | kick/kickall/ban/tempban/sudo/kill/jail exempt nodes; ban/banip/kick/jail notify |
| seen: alts by IP, IP search | done | /alts and `/seen <ip>` with `seen.ipsearch` |

## items (36 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /anvil | core |  | todo | |
| /book | core |  | todo | |
| /cartographytable | core | carttable | todo | |
| /clearinventory | core | ci, clean, clear, clearinvent | todo | |
| /clearinventoryconfirmtoggle | core | clearinventoryconfirmoff, clearconfirmoff, clearconfirmon, clearconfirm | todo | |
| /condense | core | compact, blocks, toblocks | todo | |
| /createkit | core | kitcreate, createk, kc, ck | todo | |
| /delkit | core | remkit, rmkit, deletekit | todo | |
| /disposal | core | trash | todo | |
| /enchant | core | enchantment | todo | |
| /enderchest | core | echest, endersee, ec | todo | |
| /exp | core | xp | todo | |
| /firework | core |  | todo | |
| /give | core |  | todo | |
| /grindstone | core |  | todo | |
| /hat | core | head | todo | |
| /item | core | i | todo | |
| /itemdb | core | dura, durability, itemno | todo | |
| /itemlore | core | lore, ilore | todo | |
| /itemname | core | iname, itemrename, irename | todo | |
| /kit | core | kits | todo | |
| /kitreset | core | kitr, resetkit | todo | |
| /loom | core |  | todo | |
| /more | core |  | todo | |
| /potion | core | elixer | todo | |
| /powertool | core | pt | todo | |
| /powertoollist | core | ptlist | todo | |
| /powertooltoggle | core | ptt, pttoggle | todo | |
| /recipe | core | formula, method, recipes | todo | |
| /repair | core | fix | todo | |
| /showkit | core | kitpreview, preview, kitshow | todo | |
| /skull | core | playerskull, head | todo | |
| /smithingtable | core | smithtable | todo | |
| /stonecutter | core |  | todo | |
| /unlimited | core | ul, unl | todo | |
| /workbench | core | craft, wb, wbench | todo | |

**Features (items)**

| Feature | Status | Notes |
|---|---|---|
| kits.yml format (items, meta, commands), delays, one-time kits, kit-auto-equip, drop-items-if-full | todo | |
| items.json item aliases (itemdb) | todo | |
| unlimited items (no consumption on place) | todo | |
| powertools: commands on interact ({player}) | todo | |
| item-spawn-blacklist, permission-based-item-spawn | todo | |
| oversized stacks | todo | |
| virtual containers: disposal, kit preview, enderchest of others | todo | |
| portable workstations (no block) | todo | |

## economy (9 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /balance | core | bal, money | todo | |
| /balancetop | core | baltop | todo | |
| /eco | core | economy | todo | |
| /pay | core |  | todo | |
| /payconfirmtoggle | core | payconfirmoff, payconfirmon, payconfirm | todo | |
| /paytoggle | core | payoff, payon | todo | |
| /sell | core |  | todo | |
| /setworth | core |  | todo | |
| /worth | core | price | todo | |

**Features (economy)**

| Feature | Status | Notes |
|---|---|---|
| balances, starting-balance, min-money / max-money / eco.loan | todo | |
| currency-symbol (+suffix) formatting | todo | |
| economy-log (CSV) | todo | |
| worth.yml, sell-multipliers, allow-bulk-buy-sell, allow-selling-named-items | todo | |
| baltop cache, baltop-requirements, entry limit | todo | |
| pay confirmation, minimum-pay-amount, pay-excludes-ignore-list | todo | |
| command-costs (incl. chat-shout/question), nocommandcost | todo | |
| signs: [Balance] [Buy] [Sell] [Trade] [Free] [Disposal] [Enchant] [Repair] [Heal] [Gamemode] [Info] [Kit] [Mail] [RandomTeleport] [Warp] [Anvil] [Cartography] [Grindstone] [Loom] [Smithing] [Workbench] [Spawnmob] [Time] [Weather] [Protection], colour signs, sign-use-per-second, enabledSigns | todo | |

## world (36 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /antioch | core | grenade, tnt | todo | |
| /beezooka | core | beecannon | todo | |
| /bigtree | core | largetree | todo | |
| /break | core |  | todo | |
| /burn | core |  | todo | |
| /editsign | core | sign | todo | |
| /ext | core | extinguish | todo | |
| /feed | core | eat | todo | |
| /fireball | core | fireentity, fireskull | todo | |
| /fly | core |  | todo | |
| /gamemode | core | adventure, adventuremode, creative, creativemode, gm, gma, gmc, gms, gmt, survival, survivalmode, gmsp, sp, spec, spectator | todo | |
| /god | core | godmode, tgm | todo | |
| /heal | core |  | todo | |
| /ice | core | efreeze | todo | |
| /kittycannon | core |  | todo | |
| /lightning | core | shock, smite, strike, thor | todo | |
| /nuke | core |  | todo | |
| /ptime | core | playertime | todo | |
| /pweather | core | playerweather | todo | |
| /remove | core | butcher, killall, mobkill | todo | |
| /rest | core |  | todo | |
| /spawner | core | changems, mobspawner | todo | |
| /spawnmob | core | mob, spawnentity | todo | |
| /speed | core | flyspeed, fspeed, walkspeed, wspeed | todo | |
| /suicide | core |  | todo | |
| /thunder | core |  | todo | |
| /time | core | day, night | todo | |
| /tree | core |  | todo | |
| /weather | core | rain, sky, storm, sun | todo | |
| /discord | discord |  | todo | |
| /discordbroadcast | discord | dbroadcast, dbc, dbcast | todo | |
| /link | discordlink | discordlink | todo | |
| /unlink | discordlink | discordunlink | todo | |
| /setxmpp | xmpp |  | todo | |
| /xmpp | xmpp |  | todo | |
| /xmppspy | xmpp |  | todo | |

**Features (world)**

| Feature | Status | Notes |
|---|---|---|
| god mode (no damage, no hunger) | todo | |
| fly: persistence, safelogin, world-change reset | todo | |
| speed caps (max-walk-speed, max-fly-speed) | todo | |
| death: vanishing/binding items policy, keepinv, keepxp | todo | |
| Protect module: fire, lava, explosions, mob spawns, damage types | todo | |
| AntiBuild module: build/break/use/drop/pickup permissions, blacklists, alerts | todo | |
| GeoIP: database, join message, whois line | todo | |
| Discord: chat both ways, events, console relay, commands | todo | |
| DiscordLink: link codes, role sync, unlinked kick | todo | |
| XMPP bridge | todo | |

Total: 163 commands.
