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
