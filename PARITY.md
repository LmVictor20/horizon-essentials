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
| /back | core | return | todo | |
| /bottom | core |  | todo | |
| /delhome | core | remhome, rmhome | todo | |
| /delwarp | core | remwarp, rmwarp | todo | |
| /home | core | homes | todo | |
| /jump | core | j, jumpto | todo | |
| /renamehome | core |  | todo | |
| /sethome | core | createhome | todo | |
| /settpr | core | settprandom | todo | |
| /setwarp | core | createwarp | todo | |
| /top | core |  | todo | |
| /tp | core | tele, teleport, tp2p | todo | |
| /tpa | core | call, tpask | todo | |
| /tpaall | core |  | todo | |
| /tpacancel | core |  | todo | |
| /tpaccept | core | tpyes | todo | |
| /tpahere | core |  | todo | |
| /tpall | core |  | todo | |
| /tpauto | core |  | todo | |
| /tpdeny | core | tpno | todo | |
| /tphere | core | s | todo | |
| /tpo | core |  | todo | |
| /tpoffline | core | otp, offlinetp, tpoff, tpoffline | todo | |
| /tpohere | core |  | todo | |
| /tppos | core |  | todo | |
| /tpr | core | tprandom | todo | |
| /tptoggle | core |  | todo | |
| /warp | core | warps | todo | |
| /warpinfo | core |  | todo | |
| /world | core |  | todo | |
| /setspawn | spawn |  | todo | |
| /spawn | spawn |  | todo | |

**Features (tp)**

| Feature | Status | Notes |
|---|---|---|
| teleport-delay warmup, cancelled by moving or damage (essentials.teleport.timer.*) | todo | |
| teleport-cooldown and its bypasses | todo | |
| teleport-safety / force-safe-teleport-location / is-water-safe | todo | |
| teleport-invulnerability | todo | |
| teleport-to-center | todo | |
| /back on teleport and on death (back.ondeath, back.into.<world>) | todo | |
| tpa-accept-cancellation, tpa-max-requests | todo | |
| sethome-multiple groups and limits, confirm-home-overwrite, world-home-permissions | todo | |
| per-warp-permission, warp list paging | todo | |
| Spawn module: spawn-on-join, newbies (spawnpoint, kit, announce-format), respawn-at-home(-bed), respawn-at-anchor | todo | |
| random-spawn-location, random-respawn-location, tpr.yml areas and excluded biomes | todo | |
| world-teleport-permissions | todo | |

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
| /backup | core |  | todo | |
| /ban | core |  | todo | |
| /banip | core |  | todo | |
| /compass | core | direction | todo | |
| /deljail | core | remjail, rmjail | todo | |
| /depth | core | height | todo | |
| /essentials | core | ess, essversion | todo | |
| /gc | core | lag, mem, memory, uptime, tps, entities | todo | |
| /getpos | core | coords, position, whereami, getlocation, getloc | todo | |
| /invsee | core |  | todo | |
| /jailedplayers | core | ejailed, ejp | todo | |
| /jails | core |  | todo | |
| /kick | core |  | todo | |
| /kickall | core |  | todo | |
| /kill | core |  | todo | |
| /list | core | online, playerlist, plist, who | todo | |
| /near | core | nearby | todo | |
| /playtime | core |  | todo | |
| /seen | core | alts | todo | |
| /setjail | core | createjail | todo | |
| /sudo | core |  | todo | |
| /tempban | core |  | todo | |
| /tempbanip | core |  | todo | |
| /togglejail | core | jail, tjail, unjail | todo | |
| /unban | core | pardon | todo | |
| /unbanip | core | pardonip | todo | |
| /vanish | core | v | todo | |
| /whois | core |  | todo | |

**Features (admin)**

| Feature | Status | Notes |
|---|---|---|
| permissions provider: permissions.yml groups, inheritance, wildcards, negation, prefix/suffix | todo | |
| ban/tempban/ipban enforced at login | todo | |
| jail: restrictions (break/place/interact/commands), jail-online-time, teleport-when-freed | todo | |
| vanish: tab list, entity, /list, status sample | todo | |
| invsee: live view and edits | todo | |
| disabled-commands, overridden-commands, command-cooldowns (+persistence) | todo | |
| log-console-commands | todo | |
| kickall/kick exempt nodes, ban notify | todo | |
| seen: alts by IP, IP search | todo | |

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
