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
