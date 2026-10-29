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
| /anvil | core |  | done | portable anvil (as_guest + open_station; block checks skipped while open); L3 items_give |
| /book | core |  | done | sign/unsign, author, title; book.others/author/title perms |
| /cartographytable | core | carttable | done | portable station |
| /clearinventory | core | ci, clean, clear, clearinvent | done | *, **, items, amounts, others/all, confirm prompt (confirm-clear) |
| /clearinventoryconfirmtoggle | core | clearinventoryconfirmoff, clearconfirmoff, clearconfirmon, clearconfirm | done | on/off labels; userdata confirm-clear, default-enabled-confirm-commands |
| /condense | core | compact, blocks, toblocks | done | over vanilla recipe JSON from Horizon (square 2x2/3x3, reverse check) |
| /createkit | core | kitcreate, createk, kc, ck | partial | writes kits.yml (slot:N lines, item meta); pastebin-createkit has no paste service: written locally; NBT (@base64) kits unsupported |
| /delkit | core | remkit, rmkit, deletekit | done | kits.yml and kits/*.yml |
| /disposal | core | trash | done | virtual 4-row container, items discarded on close; L3 items_give |
| /enchant | core | enchantment | done | names/aliases from Enchantments.java, safe/unsafe levels, can_enchant from Horizon |
| /enderchest | core | echest, endersee, ec | done | own: portable ender chest; others: virtual copy written back on close, read-only without essentials.enderchest.modify (click policy) |
| /exp | core | xp | done | show/set/give/take/reset, `l` levels, others; uses world area exp Host methods |
| /firework | core |  | partial | clear/power/meta work; fire spawns rockets via Host::spawn_entity (world area) with FireworksItem SNBT, directional velocity untested in-game |
| /give | core |  | done | *, oversized, default-stack-size, meta, drop-items-if-full / giveSpawnFailure; L3 items_give |
| /grindstone | core |  | done | portable station |
| /hat | core | head | done | prevent-type (exact perms), binding curse, remove |
| /item | core | i | done | amount, meta, blacklist/permission spawn, oversized; L3 items_give |
| /itemdb | core | dura, durability, itemno | done | items.json names/aliases + custom_items.yml, durability |
| /itemlore | core | lore, ilore | done | add/set/clear, max-itemlore-lines, colour perms; lore sent to clients by a Hub::stack hook |
| /itemname | core | iname, itemrename, irename | partial | Horizon keeps custom_name as a plain string: colours travel as § codes (vanilla clients draw them), no hover/click styles |
| /kit | core | kits | done | kits.yml + kits/*.yml, delays, one-time, list (struck when used), others, console; kit costs belong to economy (not charged); L3 items_give |
| /kitreset | core | kitr, resetkit | done |  |
| /loom | core |  | done | portable station |
| /more | core |  | done | oversized aware |
| /potion | core | elixer | partial | custom effects are written but Horizon keeps only potion_contents.potion (custom effects dropped on save/send); apply uses vanilla /effect; effect perm nodes use vanilla ids (essentials.potions.slowness, not slow) |
| /powertool | core | pt | done | l:/d:/r:/a:, c: chat, {player}; console form; fired from Punch (left click) and guest_attack hooks |
| /powertoollist | core | ptlist | done |  |
| /powertooltoggle | core | ptt, pttoggle | done |  |
| /recipe | core | formula, method, recipes | partial | text output for everyone (EssentialsX shows players a workbench view with the ingredients) |
| /repair | core | fix | done | hand/all, armor perm, repair-enchanted; repair costs belong to economy (not charged) |
| /showkit | core | kitpreview, preview, kitshow | done | text list like EssentialsX |
| /skull | core | playerskull, head | done | names, texture hashes and base64 textures; profile sent to clients by the Hub::stack hook; others check on the receiver as EssentialsX |
| /smithingtable | core | smithtable | done | portable station |
| /stonecutter | core |  | done | portable station |
| /unlimited | core | ul, unl | done | list/clear/toggle, others, permission-based-item-spawn; refill on block use (see feature row) |
| /workbench | core | craft, wb, wbench | done | portable crafting table (lan_open_crafting at VEHICLE_KEY_Y, validity hook); L3 items_give |

**Features (items)**

| Feature | Status | Notes |
|---|---|---|
| kits.yml format (items, meta, commands), delays, one-time kits, kit-auto-equip, drop-items-if-full | done | items with EssentialsX meta (name, lore, enchantments, colour, player, book, firework, potion, banner, trim, itemflags, unbreakable, custom-model-data, [components]), slot:, /commands ({USERNAME}/{PLAYER}), $money via /eco, delays, one-time, kit-auto-equip, drop-items-if-full, essentials.kits.<name>; pub fn ess_items::give_kit for other areas |
| items.json item aliases (itemdb) | done | generated by tools/items_db.py (1931 items, potions, spawners) + custom_items.yml + any registry id |
| unlimited items (no consumption on place) | partial | block placement only (survival): held stack snapshotted before UseItemOn and restored next tick if spent; eggs and buckets (UseItem) not hooked |
| powertools: commands on interact ({player}) | partial | left clicks (air and dig start) and hits on players fire; the dig itself is not cancelled (no macro-free hook on PlayerAction) |
| item-spawn-blacklist, permission-based-item-spawn | done |  |
| oversized stacks | done | essentials.oversizedstacks + oversized-stacksize in /give, /item, /more, kits |
| virtual containers: disposal, kit preview, enderchest of others | done | Host::open_container via stand-in chest keys at VEHICLE_KEY_Y; click policy per open (container_click asked once per raw slot, Host::opened_containers/set_container_policy), container_close via Host::closed_containers each tick; kit preview is text (/showkit) as in EssentialsX |
| portable workstations (no block) | done | all seven + ender chest |

## economy (9 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /balance | core | bal, money | done | L1 + L3 (economy_money). Display names: nickname (+nickname-prefix) else name. |
| /balancetop | core | baltop | done | L1 + L3. 2-minute cache, `force`, 9-line pages, orderBalances over 50 users; computed synchronously. |
| /eco | core | economy | done | give/take/set/reset, `*`/`**`/uuid/offline targets; L1 + L3. `10%` shows the usage, as EssentialsX's parser does. |
| /pay | core |  | done | Offline/hidden, `**`, pay.multiple, confirmation, ignore list, max-money refund; L1 + L3. |
| /payconfirmtoggle | core | payconfirmoff, payconfirmon, payconfirm | done | userdata `confirm-pay`, default-enabled-confirm-commands. |
| /paytoggle | core | payoff, payon | done | userdata `accepting-pay`, others; L1 + L3. |
| /sell | core |  | done | hand/item/all/blocks, amounts (-1, 2s), trade-in-stacks, multipliers, named items, console log. Item names: ids, underscore-less ids, custom_items.yml (no items.csv aliases). |
| /setworth | core |  | done | Writes worth.yml (its comments are not kept). |
| /worth | core | price | done | worthMeta for damaged items. |

**Features (economy)**

| Feature | Status | Notes |
|---|---|---|
| balances, starting-balance, min-money / max-money / eco.loan | done | Exact decimals (BigDecimal semantics, scale kept: `50.00`), userdata `money` as a string; disable-eco. |
| currency-symbol (+suffix) formatting | done | NumberUtil: currency-format, FLOOR rounding, `.00` dropped, currency-symbol-format-locale for common languages. |
| economy-log (CSV) | done | trade.log lines as Trade.log (incl. update/uuid options); date in the server's locale-free English form. |
| worth.yml, sell-multipliers, allow-bulk-buy-sell, allow-selling-named-items | partial | All in L1. In game, bulk [Buy]/[Sell] while sneaking with the item never reaches the sign (vanilla uses the held item instead of opening the sign: see CORE-REQUESTS 3). |
| baltop cache, baltop-requirements, entry limit | done | minimum-playtime uses the play_time statistic of online players; offline players skip it (EssentialsX before 1.15.2 does the same). |
| pay confirmation, minimum-pay-amount, pay-excludes-ignore-list | done | |
| command-costs (incl. chat-shout/question), nocommandcost | partial | Charged before the command runs (no post-command hook: CORE-REQUESTS 1), so a failing command still costs. `ess_economy::charge_command_cost(ess, host, player, "chat-shout")` for other areas. |
| signs: [Balance] [Buy] [Sell] [Trade] [Free] [Disposal] [Enchant] [Repair] [Heal] [Gamemode] [Info] [Kit] [Mail] [RandomTeleport] [Warp] [Anvil] [Cartography] [Grindstone] [Loom] [Smithing] [Workbench] [Spawnmob] [Time] [Weather] [Protection], colour signs, sign-use-per-second, enabledSigns | partial | All 25 signs, create/use/break permissions, [cost] syntax ($, items, exp, times), colours, throttle, unprotected-sign-names, explosion/fire protection: L1 (57 tests). Game glue: sign edits get sign_change on the next tick (a refused edit gets its old text back); right clicks on unwaxed signs work (on the next tick), waxed signs and sneaking clicks don't reach Essentials; breaking and container opening are decided with a last-tick player snapshot (CORE-REQUESTS 2/3). [Kit]/[Mail]/[Info] run `/kit`, `/mail read`, `/info` as the player (their permissions apply); [Warp]/[Spawnmob]/[RandomTeleport] run `/warp`, `/spawnmob`, `/tpr` from the console. [Free] ignores item meta lines; [Enchant] has no item tags/aliases and doesn't check that an enchantment fits the item. Owner data is the name on the sign (no PDC). Signs in levels not in place are unreachable. No L3 (ess-bot can't place or click signs). |

## world (36 commands)

| Command | Module | Aliases | Status | Notes |
|---|---|---|---|---|
| /antioch | core | grenade, tnt | done | Primed TNT at the target block; with an argument the two broadcast lines. |
| /beezooka | core | beecannon | partial | Bee launched from the eyes (Motion NBT); after 20 ticks a zero-radius creeper explosion at it, then the bee is removed with /kill (death animation, unlike Bukkit remove()). |
| /bigtree | core | largetree | done | tree/redwood/jungle/darkoak -> vanilla configured features via `place feature`; solid target -> bigTreeFailure. |
| /break | core |  | done | Target block (20) to air; bedrock needs essentials.break.bedrock; block_break listeners (AntiBuild) may veto. |
| /burn | core |  | done |  |
| /editsign | core | sign | done | set/clear/copy/paste, line numbers, 15-char limit (editsign.unlimited), & codes per editsign.color/format/magic, sign_change listeners. Front side only; waxed signs not detected. |
| /ext | core | extinguish | done |  |
| /feed | core | eat | done | Shares the heal cooldown (timestamps.last-heal). L3 world_player. |
| /fireball | core | fireentity, fireskull | partial | All 13 types with essentials.fireball.<type>, speed capped by max-projectile-speed, Motion/Owner NBT; `ride` uses vanilla /ride (unverified in Horizon). |
| /fly | core |  | done | Toggle/others/on-off, flymode userdata; survival flight via the abilities flags and the client toggles (hooks on Guest::ability_flags, Hub::tick_level). L3 checks messages only (ess-bot cannot read abilities). |
| /gamemode | core | adventure, adventuremode, creative, creativemode, gm, gma, gmc, gms, gmt, survival, survivalmode, gmsp, sp, spec, spectator | done | All labels (gmc/gms/gma/gmsp/gmt...), per-mode permissions, others, toggle. L3 world_player (execute if entity). |
| /god | core | godmode, tgm | done | godmode userdata, heal on enable, no damage (cancellable HEAD on PlayerMotion::hurt_caused_by; L3: survives lava), no hunger, no-god-in-worlds, remove-god-on-disconnect. God attacker vs essentials.god.pvp: L1 only (no attacker in the hook). |
| /heal | core |  | done | heal-cooldown + essentials.heal.cooldown.bypass, remove-effects-on-heal (effect clear), healDead. Remaining air not reset. L3 world_player. |
| /ice | core | efreeze | done | Freeze ticks 140. |
| /kittycannon | core |  | partial | Baby tamed cat (random variant) launched; zero-radius explosion after 20 ticks, removal via /kill. |
| /lightning | core | shock, smite, strike, thor | partial | Strike at target / smite players (effect-only bolt + vanilla /damage with power, god exempt, warn-on-smite). In another dimension the bolt is a real /summon lightning_bolt. |
| /nuke | core |  | done | 25 primed TNT 64 blocks above each target. |
| /ptime | core | playertime | done | Relative and @fixed, get/list/reset, others; per-player SetTime clock packets, refreshed every second. |
| /pweather | core | playerweather | done | storm/sun/reset, get, others; per-player rain game events, refreshed every second. |
| /remove | core | butcher, killall, mobkill | partial | All ToRemove kinds, custom mob names, radius, world; done with vanilla /kill per type (drops loot, unlike Bukkit remove()); tamed/named mobs are not skipped in the game (L1 only). |
| /rest | core |  | done | Resets minecraft:custom/minecraft:time_since_rest. |
| /spawner | core | changems, mobspawner | done | Spawner target, mob permissions, protect prevent-spawn, essentials.spawner.delay; via data merge block. |
| /spawnmob | core | mob, spawnentity | partial | Names/suffixes, counts, spawnmob-limit, stacking (Passengers), per-mob permissions, protect prevent-spawn, target player; MobData subset (baby/adult, colors, tame, saddle, angry, charged, sizes, villager profession/type, cat/parrot/horse variants, swords, armor). Rarer data ignored. |
| /speed | core | flyspeed, fspeed, walkspeed, wspeed | done | walk/fly, labels, caps (max-walk-speed/max-fly-speed, speed.bypass), speed.fly/walk, others. Walk via the movement_speed base, fly via the abilities packet. |
| /suicide | core |  | done | Health 0, suicideMessage, suicideSuccess broadcast. |
| /thunder | core |  | done |  |
| /time | core | day, night | done | Show/set/add, day/night labels, worlds/all, time.set, world-time-permissions, DescParseTickFormat; relative ptimes kept. L3 world_player. |
| /tree | core |  | done | Every TreeType name -> vanilla configured feature via `place feature`. |
| /weather | core | rain, sky, storm, sun | done | Player and console forms, durations. L3 world_player. |
| /discord | discord |  | done | Invite link from discord-url. |
| /discordbroadcast | discord | dbroadcast, dbc, dbcast | done | Channel by name or id, per-channel/markdown/ping permissions, REST post. |
| /link | discordlink | discordlink | done | In-game code; on Discord the message command "/link CODE" in a mapped channel (deleted after use). |
| /unlink | discordlink | discordunlink | done | Also the "/unlink" message command; unsyncs roles and groups. |
| /setxmpp | xmpp |  | done | Stores the address in xmpp/users.yml; players only. |
| /xmpp | xmpp |  | done | Sends through the XMPP client; "Error sending message." when not connected. |
| /xmppspy | xmpp |  | done | Toggles spy; * / ** / @s. |

**Features (world)**

| Feature | Status | Notes |
|---|---|---|
| god mode (no damage, no hunger) | done | Damage hook on PlayerMotion::hurt_caused_by (L3 lava), hunger refilled each tick. Attacker-side god.pvp rule is L1 only. |
| fly: persistence, safelogin, world-change reset | done | flymode restored on join, fly.safelogin (shouldFly), send-fly-enable-on-join, world-change-fly-reset, world-change-preserve-flying, gamemode-change-preserve-flying. |
| speed caps (max-walk-speed, max-fly-speed) | done | Caps in /speed, reset on join without essentials.speed, world-change-speed-reset. |
| death: vanishing/binding items policy, keepinv, keepxp | partial | L1 complete; in game a HEAD hook on guest_forget_digging (first macro-free call in guest_die) keeps inventory/xp until respawn using rules cached every second; not L3-tested. |
| Protect module: fire, lava, explosions, mob spawns, damage types | partial | Every prevent/disable rule decided in L1 (damage, world events, weather, spawn, transformations, entity target, pistons/dispensers). In game only player damage is enforced (per vanilla damage type, mirrored onto the body each second); world events (fire, flow, explosions, spawns) have no Horizon hook yet; PvP rules need the attacker (CORE-REQUESTS 3). |
| AntiBuild module: build/break/use/drop/pickup permissions, blacklists, alerts | partial | All rules in L1 (place/break/interact, attached signs, drop/pickup/craft, blacklists, alerts). In game only /break emits block_break; place/interact/drop/pickup need core events (CORE-REQUESTS 1, 2). |
| GeoIP: database, join message, whois line | partial | Own .mmdb reader (record sizes 24/28/32), join message, whois_line for admin. Auto-download impossible (binary .tar.gz over a text Host HTTP API): place the .mmdb in essentials/geoip/. Off and silent without a database. |
| Discord: chat both ways, events, console relay, commands | partial | REST only: MC->Discord chat/join/quit/death/start (webhooks for name/avatar), Discord->MC polling, message commands execute/msg/list, console command relay. No gateway (presence, slash commands), no console log relay; afk/kick/mute/me/msg/advancement relays need other areas to call the hooks. Not verified against real Discord. |
| DiscordLink: link codes, role sync, unlinked kick | partial | Link codes, accounts.json, kick (just after join) / freeze policies, block-unlinked-chat, mail DMs, role<->group sync via REST + permissions.yml. No member-leave events (gateway). |
| XMPP bridge | partial | Own client (SASL PLAIN or iq:auth, bind, session, presence, chat relay, op-user commands) over plain TCP. No STARTTLS (horizon-client has no TLS socket API), so TLS-only servers are refused with a logged reason; no log forwarding. |

Total: 163 commands.
