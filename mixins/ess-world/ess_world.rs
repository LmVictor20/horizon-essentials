// horizon-essentials area `world` (pure logic, standalone-tested): Player, world & modules: fly, god, heal, feed, rest, speed, gamemode, ptime, pweather, suicide, ext, time, weather, thunder, tree, bigtree, break, spawnmob, spawner, remove, editsign, fun commands, Protect, AntiBuild, Discord, DiscordLink, XMPP, GeoIP.
// Owned by one team member; register commands and listeners in `register`.
//
// Ports of EssentialsX's Command<name>.java for this area, User god/fly state (userdata `godmode`, `flymode`,
// `timestamps.last-heal`), the fly/god/speed parts of EssentialsPlayerListener and the death and god parts of
// EssentialsEntityListener. Protect + AntiBuild live in ess_protect, Discord in ess_discord, GeoIP in ess_geoip,
// XMPP in ess_xmpp.
//
// Vocabulary the glue uses for `Ess::on_damage` causes: `fall`, `fire`, `fire_tick`, `lava`, `drowning`,
// `suffocation`, `contact`, `lightning`, `wither`, `magic`, `void`, `starvation`, `generic`,
// `explosion:<entity>` (tnt, creeper, tnt_minecart, fireball, wither_skull, end_crystal, bed...),
// `projectile:<shooter uuid or ->`, `player:<attacker uuid>`, `entity:<entity id>`.

use crate::game::ess_core::*;
use crate::game::ess_text;
use serde_json::json;
use std::collections::HashMap;

pub fn register(ess: &mut Ess) {
    let cmds: &[(&'static str, CmdFn, &'static [(&'static str, &'static str)])] = &[
        ("antioch", cmd_antioch, &[]),
        ("beezooka", cmd_beezooka, &[]),
        ("bigtree", cmd_bigtree, &[("/<command> <tree|redwood|jungle|darkoak>", "bigtreeCommandUsage1Description")]),
        ("break", cmd_break, &[]),
        ("burn", cmd_burn, &[("/<command> <player> <seconds>", "burnCommandUsage1Description")]),
        (
            "editsign",
            cmd_editsign,
            &[
                ("/<command> set <line number> <text>", "editsignCommandUsage1Description"),
                ("/<command> clear <line number>", "editsignCommandUsage2Description"),
                ("/<command> copy [line number]", "editsignCommandUsage3Description"),
                ("/<command> paste [line number]", "editsignCommandUsage4Description"),
            ],
        ),
        ("ext", cmd_ext, &[("/<command> [player]", "extCommandUsage1Description")]),
        ("feed", cmd_feed, &[("/<command> [player]", "feedCommandUsage1Description")]),
        (
            "fireball",
            cmd_fireball,
            &[
                ("/<command>", "fireballCommandUsage1Description"),
                ("/<command> <fireball|small|large|arrow|skull|egg|snowball|expbottle|dragon|splashpotion|lingeringpotion|trident> [speed]", "fireballCommandUsage2Description"),
            ],
        ),
        ("fly", cmd_fly, &[("/<command> [player]", "flyCommandUsage1Description")]),
        ("gamemode", cmd_gamemode, &[("/<command> <survival|creative|adventure|spectator> [player]", "gamemodeCommandUsage1Description")]),
        ("god", cmd_god, &[("/<command> [player]", "godCommandUsage1Description")]),
        ("heal", cmd_heal, &[("/<command> [player]", "healCommandUsage1Description")]),
        (
            "ice",
            cmd_ice,
            &[("/<command>", "iceCommandUsage1Description"), ("/<command> <player>", "iceCommandUsage2Description"), ("/<command> *", "iceCommandUsage3Description")],
        ),
        ("kittycannon", cmd_kittycannon, &[]),
        (
            "lightning",
            cmd_lightning,
            &[("/<command> [player]", "lightningCommandUsage1Description"), ("/<command> <player> <power>", "lightningCommandUsage2Description")],
        ),
        ("nuke", cmd_nuke, &[("/<command> [players...]", "nukeCommandUsage1Description")]),
        (
            "ptime",
            cmd_ptime,
            &[
                ("/<command> list [player|*]", "ptimeCommandUsage1Description"),
                ("/<command> <time> [player|*]", "ptimeCommandUsage2Description"),
                ("/<command> reset [player|*]", "ptimeCommandUsage3Description"),
            ],
        ),
        (
            "pweather",
            cmd_pweather,
            &[
                ("/<command> list [player|*]", "pweatherCommandUsage1Description"),
                ("/<command> <storm|sun> [player|*]", "pweatherCommandUsage2Description"),
                ("/<command> reset [player|*]", "pweatherCommandUsage3Description"),
            ],
        ),
        (
            "remove",
            cmd_remove,
            &[("/<command> <mob type> [world]", "removeCommandUsage1Description"), ("/<command> <mob type> <radius> [world]", "removeCommandUsage2Description")],
        ),
        ("rest", cmd_rest, &[("/<command> [player]", "restCommandUsage1Description")]),
        ("speed", cmd_speed, &[("/<command> <speed>", "speedCommandUsage1Description"), ("/<command> <type> <speed> [player]", "speedCommandUsage2Description")]),
        ("spawner", cmd_spawner, &[("/<command> <mob> [delay]", "spawnerCommandUsage1Description")]),
        (
            "spawnmob",
            cmd_spawnmob,
            &[
                ("/<command> <mob>[:data] [amount] [player]", "spawnmobCommandUsage1Description"),
                ("/<command> <mob>[:data],<mount>[:data] [amount] [player]", "spawnmobCommandUsage2Description"),
            ],
        ),
        ("suicide", cmd_suicide, &[]),
        ("thunder", cmd_thunder, &[("/<command> <true|false> [duration]", "thunderCommandUsage1Description")]),
        (
            "time",
            cmd_time,
            &[
                ("/<command>", "timeCommandUsage1Description"),
                ("/<command> set <time> [world|all]", "timeCommandUsage2Description"),
                ("/<command> add <time> [world|all]", "timeCommandUsage3Description"),
            ],
        ),
        (
            "tree",
            cmd_tree,
            &[("/<command> <tree|birch|redwood|redmushroom|brownmushroom|jungle|junglebush|swamp|paleoak>", "treeCommandUsage1Description")],
        ),
        ("weather", cmd_weather, &[("/<command> <storm|sun> [duration]", "weatherCommandUsage1Description")]),
    ];
    for (name, run, usages) in cmds {
        ess.register(Command { name, run: *run, tab: None, usages });
    }
    ess.listeners.join.push(on_join);
    ess.listeners.quit.push(on_quit);
    ess.listeners.moved.push(on_moved);
    ess.listeners.tick.push(on_tick);
    ess.listeners.damage.push(on_damage);

    crate::game::ess_protect::register(ess);
    crate::game::ess_discord::register(ess);
    crate::game::ess_geoip::register(ess);
    crate::game::ess_xmpp::register(ess);
}

// ===== Shared helpers ===============================================================================

/// In-memory state of this area.
#[derive(Default)]
struct WorldState {
    /// /editsign copy buffers.
    sign_copy: HashMap<Uuid, [String; 4]>,
    /// Per-player time as Bukkit keeps it: (offset or fixed time, relative).
    ptime: HashMap<Uuid, (i64, bool)>,
    pweather: HashMap<Uuid, bool>,
    /// (allow_flight, flying) seen last tick, for world-change-preserve-flying.
    flight: HashMap<Uuid, (bool, bool)>,
    /// Projectiles of /beezooka and /kittycannon still in the air: (fire at tick, world, tag, entity kind).
    projectiles: Vec<(u64, String, String, String)>,
    next_tag: u64,
}

/// Bukkit's world name for a dimension id (`minecraft:overworld` -> `world`).
pub fn world_name(id: &str) -> String {
    match id {
        "minecraft:overworld" => "world".into(),
        "minecraft:the_nether" => "world_nether".into(),
        "minecraft:the_end" => "world_the_end".into(),
        other => other.strip_prefix("minecraft:").unwrap_or(other).to_string(),
    }
}

/// A loaded world by Bukkit name or dimension id.
pub fn find_world(host: &mut dyn Host, name: &str) -> Option<String> {
    let worlds = host.worlds();
    worlds
        .iter()
        .find(|w| world_name(w).eq_ignore_ascii_case(name) || w.eq_ignore_ascii_case(name))
        .or_else(|| worlds.iter().find(|w| world_id(name).eq_ignore_ascii_case(w)))
        .cloned()
}

/// user.getDisplayName(): the nickname (with the configured prefix) or the name.
pub fn display_name(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    let name = host.name(player).or_else(|| ess.users.get(player).and_then(|u| u.name())).unwrap_or_else(|| uuid_string(player));
    match ess.users.get(player).and_then(|u| u.str("nickname")) {
        Some(nick) if !nick.is_empty() && nick != name => {
            let prefix = ess.config_str("nickname-prefix").unwrap_or_else(|| "~".into());
            format!("{prefix}{}", ess_text::strip_legacy(&nick, true))
        }
        _ => name,
    }
}

fn dn(ctx: &mut Ctx, player: Uuid) -> Arg {
    Arg::Plain(display_name(ctx.ess, ctx.host, player))
}

/// Online players, by name (EssentialsX keeps join order; a name order keeps output stable).
fn online_sorted(host: &mut dyn Host) -> Vec<Uuid> {
    let mut list: Vec<(String, Uuid)> = host.online().into_iter().map(|u| (host.name(u).unwrap_or_default().to_lowercase(), u)).collect();
    list.sort();
    list.into_iter().map(|(_, u)| u).collect()
}

/// Players the sender may act on (vanished ones only for `essentials.vanish.interact`).
fn visible(ctx: &mut Ctx, player: Uuid) -> bool {
    match ctx.sender {
        Sender::Player(me) if me != player => !ctx.ess.is_vanished(player) || ctx.has("essentials.vanish.interact"),
        _ => true,
    }
}

/// Bukkit's Server.matchPlayer: the exact name, else every name containing the text.
fn match_players(ctx: &mut Ctx, search: &str) -> Vec<Uuid> {
    let search_l = search.to_lowercase();
    let online = online_sorted(ctx.host);
    let mut out = Vec::new();
    for p in online {
        let name = ctx.host.name(p).unwrap_or_default().to_lowercase();
        if name == search_l {
            return if visible(ctx, p) { vec![p] } else { Vec::new() };
        }
        if name.contains(&search_l) && visible(ctx, p) {
            out.push(p);
        }
    }
    out
}

/// EssentialsLoopCommand.loopOnlinePlayersConsumer: the players a search names.
fn loop_online(ctx: &mut Ctx, multiple: bool, wildcards: bool, search: &str) -> Result<Vec<Uuid>, CmdError> {
    if search.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    if let Sender::Player(me) = ctx.sender {
        if search == "@s" || search == "@p" {
            return Ok(vec![me]);
        }
    }
    if wildcards && (search == "*" || search == "**") {
        let all = online_sorted(ctx.host);
        return Ok(all.into_iter().filter(|p| visible(ctx, *p)).collect());
    }
    if multiple {
        if search.trim().chars().count() < 2 {
            return Err(CmdError::PlayerNotFound);
        }
        let mut found = match_players(ctx, search);
        if found.is_empty() {
            let text = search.to_lowercase();
            for p in online_sorted(ctx.host) {
                if visible(ctx, p) && ess_text::strip_legacy(&display_name(ctx.ess, ctx.host, p), true).to_lowercase().contains(&text) {
                    found.push(p);
                }
            }
        }
        if found.is_empty() {
            return Err(CmdError::PlayerNotFound);
        }
        return Ok(found);
    }
    Ok(vec![ctx.online_player(search)?])
}

/// EssentialsCommand.getPlayer(server, sender, args, pos).
fn get_player(ctx: &mut Ctx, args: &[String], pos: usize) -> Result<Uuid, CmdError> {
    let Some(search) = args.get(pos) else { return Err(CmdError::Usage(None)) };
    if search.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    ctx.online_player(search)
}

/// MessageFormat's default number pattern for a float (`#,##0.###`).
fn fmt_num(v: f64) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" { "0".into() } else { s }
}

fn tl_parsed(ctx: &Ctx, key: &str) -> Arg {
    Arg::Parsed(ctx.ess.tl(key, &[]))
}

fn enable_disable(ctx: &Ctx, on: bool) -> Arg {
    tl_parsed(ctx, if on { "enabled" } else { "disabled" })
}

/// EssentialsToggleCommand.matchToggleArgument.
fn match_toggle(arg: &str) -> Option<bool> {
    if arg.eq_ignore_ascii_case("on") || arg.starts_with("ena") || arg == "1" {
        Some(true)
    } else if arg.eq_ignore_ascii_case("off") || arg.starts_with("dis") || arg == "0" {
        Some(false)
    } else {
        None
    }
}

/// DateUtil.formatDateDiff for a future instant: up to three units ("1 minute", "2 hours 3 minutes").
pub fn format_date_diff(ess: &Ess, now: i64, until: i64) -> String {
    if until == now {
        return ess.tl("now", &[]);
    }
    let mut secs = ((until - now).abs() + 50) / 1000;
    let units = [(86400 * 365, "year", "years"), (86400 * 30, "month", "months"), (86400, "day", "days"), (3600, "hour", "hours"), (60, "minute", "minutes"), (1, "second", "seconds")];
    let mut out = Vec::new();
    for (size, one, many) in units {
        if out.len() > 2 {
            break;
        }
        let n = secs / size;
        if n > 0 {
            secs -= n * size;
            out.push(format!("{n} {}", ess.tl(if n > 1 { many } else { one }, &[])));
        }
    }
    if out.is_empty() { ess.tl("now", &[]) } else { out.join(" ") }
}

fn block_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

/// Blocks a player or entity can stand in (Bukkit's "not solid" for the materials that matter here).
pub fn is_hollow(state: &str) -> bool {
    let id = block_id(state).trim_start_matches("minecraft:");
    matches!(
        id,
        "air" | "cave_air" | "void_air" | "short_grass" | "grass" | "tall_grass" | "fern" | "large_fern" | "dead_bush" | "snow" | "water" | "lava"
            | "torch" | "wall_torch" | "redstone_wire" | "rail" | "powered_rail" | "detector_rail" | "activator_rail" | "vine" | "seagrass"
            | "tall_seagrass" | "kelp" | "kelp_plant" | "sugar_cane" | "wheat" | "carrots" | "potatoes" | "beetroots" | "lever" | "stone_button"
            | "oak_button" | "light" | "structure_void" | "cobweb" | "nether_portal" | "end_portal"
    ) || id.ends_with("_sapling")
        || id.ends_with("_flower")
        || id.ends_with("_tulip")
        || id.ends_with("_carpet")
        || id.ends_with("_sign")
        || id.ends_with("_button")
        || id.ends_with("_pressure_plate")
        || id.ends_with("_banner")
        || matches!(id, "dandelion" | "poppy" | "blue_orchid" | "allium" | "azure_bluet" | "oxeye_daisy" | "cornflower" | "lily_of_the_valley" | "brown_mushroom" | "red_mushroom")
}

/// LocationUtil.getTarget: the block the player looks at (else where they stand).
fn target(ctx: &mut Ctx, player: Uuid, max: f64) -> Loc {
    let here = ctx.host.location(player).unwrap_or_else(|| Loc::new("minecraft:overworld", 0.0, 64.0, 0.0));
    match ctx.host.target_block(player, max) {
        Some(b) => Loc { world: here.world.clone(), x: b[0] as f64, y: b[1] as f64, z: b[2] as f64, yaw: 0.0, pitch: 0.0 },
        None => Loc { world: here.world.clone(), x: here.x.floor(), y: here.y.floor(), z: here.z.floor(), yaw: 0.0, pitch: 0.0 },
    }
}

/// LocationUtil.getSafeDestination (simplified): the first space at or above `loc` a mob fits in, centered.
fn safe_destination(host: &mut dyn Host, loc: &Loc) -> Loc {
    let [x, mut y, z] = loc.block();
    let (_, max) = host.height_range(&loc.world);
    while y < max {
        let here = host.block(&loc.world, [x, y, z]).unwrap_or_else(|| "minecraft:air".into());
        let above = host.block(&loc.world, [x, y + 1, z]).unwrap_or_else(|| "minecraft:air".into());
        if is_hollow(&here) && is_hollow(&above) {
            break;
        }
        y += 1;
    }
    Loc { world: loc.world.clone(), x: x as f64 + 0.5, y: y as f64, z: z as f64 + 0.5, yaw: loc.yaw, pitch: loc.pitch }
}

/// A Bukkit UUID as the NBT int array (`[I;a,b,c,d]`).
fn uuid_ints(u: Uuid) -> String {
    let parts: Vec<String> = (0..4).map(|i| ((u >> (96 - 32 * i)) as u32 as i32).to_string()).collect();
    format!("[I;{}]", parts.join(","))
}

/// The eye location and the look direction of a player.
fn eye_and_direction(host: &mut dyn Host, player: Uuid) -> (Loc, [f64; 3]) {
    let at = host.location(player).unwrap_or_else(|| Loc::new("minecraft:overworld", 0.0, 64.0, 0.0));
    let (yaw, pitch) = ((at.yaw as f64).to_radians(), (at.pitch as f64).to_radians());
    let dir = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    let eye = Loc { y: at.y + 1.62, ..at };
    (eye, dir)
}

fn motion(dir: [f64; 3], speed: f64) -> String {
    format!("Motion:[{:.4}d,{:.4}d,{:.4}d]", dir[0] * speed, dir[1] * speed, dir[2] * speed)
}

/// Whether EssentialsX treats the player as in god mode now (User.isGodModeEnabled).
pub fn is_god(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> bool {
    if ess.users.get(player).is_some_and(|u| u.bool("godmode")) {
        let world = host.location(player).map(|l| world_name(&l.world)).unwrap_or_default();
        if !ess.config_list("no-god-in-worlds").iter().any(|w| *w == world) {
            return true;
        }
    }
    if ess.users.get(player).is_some_and(|u| u.bool("afk")) {
        return ess.config_bool("freeze-afk-players", false);
    }
    false
}

/// User.healCooldown (shared by /heal and /feed).
fn heal_cooldown(ctx: &mut Ctx, player: Uuid) -> CmdResult {
    let now = ctx.host.now_millis();
    let last = ctx.ess.users.user(player).i64("timestamps.last-heal").unwrap_or(0);
    if last > 0 {
        let cooldown = ctx.ess.config_f64("heal-cooldown", 60.0);
        let until = last + (cooldown * 1000.0) as i64;
        if until > now && !ctx.has("essentials.heal.cooldown.bypass") {
            let diff = format_date_diff(ctx.ess, now, until);
            return Err(CmdError::Tl("timeBeforeHeal", vec![Arg::Plain(diff)]));
        }
    }
    ctx.ess.users.user(player).set("timestamps.last-heal", json!(now));
    Ok(())
}

/// ess.broadcastTl(sender, key, args): every online player (not when the sender is vanished).
fn broadcast_tl(ess: &mut Ess, host: &mut dyn Host, sender: Option<Uuid>, key: &str, args: &[Arg]) {
    if sender.is_some_and(|s| ess.is_vanished(s)) {
        return;
    }
    for p in online_sorted(host) {
        ess.send_tl(host, Sender::Player(p), key, args);
    }
}

// ===== /fly, /god (EssentialsToggleCommand) =========================================================

type ToggleFn = fn(&mut Ctx, Uuid, Option<bool>) -> CmdResult;

fn toggle_command(ctx: &mut Ctx, args: &[String], others: &str, toggle: ToggleFn) -> CmdResult {
    let Sender::Player(me) = ctx.sender else { return toggle_others(ctx, args, toggle) };
    if args.len() == 1 {
        let wanted = match_toggle(&args[0]);
        if wanted.is_none() && ctx.has(others) {
            return toggle_others(ctx, args, toggle);
        }
        toggle(ctx, me, wanted)
    } else if args.len() == 2 && ctx.has(others) {
        toggle_others(ctx, args, toggle)
    } else {
        toggle(ctx, me, None)
    }
}

fn toggle_others(ctx: &mut Ctx, args: &[String], toggle: ToggleFn) -> CmdResult {
    if args.is_empty() || args[0].trim().chars().count() < 2 {
        return Err(CmdError::PlayerNotFound);
    }
    let found = match_players(ctx, &args[0]);
    if found.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    let wanted = args.get(1).and_then(|a| match_toggle(a));
    for p in found {
        toggle(ctx, p, wanted)?;
    }
    Ok(())
}

fn cmd_fly(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.fly.others", fly_player)
}

fn fly_player(ctx: &mut Ctx, player: Uuid, wanted: Option<bool>) -> CmdResult {
    let enabled = wanted.unwrap_or_else(|| !ctx.host.allow_flight(player));
    ctx.host.set_allow_flight(player, enabled);
    ctx.ess.users.user(player).set("flymode", json!(enabled));
    if !enabled {
        ctx.host.set_flying(player, false);
    }
    let args = [enable_disable(ctx, enabled), dn(ctx, player)];
    ctx.tl_to(Sender::Player(player), "flyMode", &args);
    if ctx.sender != Sender::Player(player) {
        ctx.tl("flyMode", &args);
    }
    Ok(())
}

fn cmd_god(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.god.others", god_player)
}

fn god_player(ctx: &mut Ctx, player: Uuid, wanted: Option<bool>) -> CmdResult {
    let enabled = wanted.unwrap_or_else(|| !ctx.ess.users.get(player).is_some_and(|u| u.bool("godmode")));
    ctx.ess.users.user(player).set("godmode", json!(enabled));
    let (health, max) = ctx.host.health(player);
    if enabled && health != 0.0 {
        ctx.host.set_health(player, max);
        let (_, sat) = ctx.host.food(player);
        ctx.host.set_food(player, 20, sat);
    }
    let args = [enable_disable(ctx, enabled)];
    ctx.tl_to(Sender::Player(player), "godMode", &args);
    if ctx.sender != Sender::Player(player) {
        ctx.tl("godMode", &args);
    }
    Ok(())
}

// ===== /heal /feed /ext /burn /ice /rest /suicide ====================================================

fn cmd_heal(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if let Sender::Player(me) = ctx.sender {
        if !ctx.has("essentials.heal.cooldown.bypass") {
            heal_cooldown(ctx, me)?;
        }
        if !args.is_empty() && ctx.has("essentials.heal.others") {
            for p in loop_online(ctx, true, true, &args[0])? {
                heal_player(ctx, p)?;
            }
            return Ok(());
        }
        return heal_player(ctx, me);
    }
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    for p in loop_online(ctx, true, true, &args[0])? {
        heal_player(ctx, p)?;
    }
    Ok(())
}

fn heal_player(ctx: &mut Ctx, player: Uuid) -> CmdResult {
    let (health, max) = ctx.host.health(player);
    if health == 0.0 {
        return Err(CmdError::Tl("healDead", vec![]));
    }
    ctx.host.set_health(player, max);
    let (_, sat) = ctx.host.food(player);
    ctx.host.set_food(player, 20, sat);
    ctx.host.set_fire_ticks(player, 0);
    ctx.tl_to(Sender::Player(player), "heal", &[]);
    if ctx.ess.config_bool("remove-effects-on-heal", true) {
        let name = ctx.name_of(player);
        ctx.host.console_command(&format!("effect clear {name}"));
    }
    if ctx.sender != Sender::Player(player) {
        let arg = dn(ctx, player);
        ctx.tl("healOther", &[arg]);
    }
    Ok(())
}

fn feed(host: &mut dyn Host, player: Uuid) {
    let (food, _) = host.food(player);
    host.set_food(player, (food + 30).min(20).max(20.min(food + 30)), 10.0);
}

fn cmd_feed(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else {
        if args.is_empty() {
            return Err(CmdError::Usage(None));
        }
        return feed_others(ctx, &args[0]);
    };
    if !ctx.has("essentials.feed.cooldown.bypass") {
        heal_cooldown(ctx, me)?;
    }
    if !args.is_empty() && ctx.has("essentials.feed.others") {
        return feed_others(ctx, &args[0]);
    }
    feed(ctx.host, me);
    ctx.tl("feed", &[]);
    Ok(())
}

fn feed_others(ctx: &mut Ctx, search: &str) -> CmdResult {
    for p in loop_online(ctx, true, true, search)? {
        feed(ctx.host, p);
        let arg = dn(ctx, p);
        ctx.tl("feedOther", &[arg]);
    }
    Ok(())
}

fn cmd_ext(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if let Sender::Player(me) = ctx.sender {
        if args.is_empty() || !ctx.has("essentials.ext.others") {
            ctx.host.set_fire_ticks(me, 0);
            ctx.tl("extinguish", &[]);
            return Ok(());
        }
    } else if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    for p in loop_online(ctx, true, true, &args[0])? {
        ctx.host.set_fire_ticks(p, 0);
        let arg = dn(ctx, p);
        ctx.tl("extinguishOthers", &[arg]);
    }
    Ok(())
}

fn parse_int(s: &str) -> Result<i64, CmdError> {
    s.parse::<i32>().map(i64::from).map_err(|_| CmdError::Msg(format!("For input string: \"{s}\"")))
}

fn cmd_burn(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let player = get_player(ctx, args, 0)?;
    let seconds = parse_int(&args[1])?;
    let ticks = (seconds * 20).clamp(0, i32::MAX as i64) as i32;
    ctx.host.set_fire_ticks(player, ticks);
    let arg = dn(ctx, player);
    ctx.tl("burnMsg", &[arg, Arg::Plain(seconds.max(0).to_string())]);
    Ok(())
}

/// Vanilla's maximum freeze ticks (Entity.getTicksRequiredToFreeze).
const MAX_FREEZE: i32 = 140;

fn cmd_ice(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() && !ctx.is_player() {
        return Err(CmdError::Usage(None));
    }
    if !args.is_empty() && ctx.has("essentials.ice.others") {
        for p in loop_online(ctx, false, true, &args[0])? {
            ctx.host.set_freeze_ticks(p, MAX_FREEZE);
            ctx.tl_to(Sender::Player(p), "ice", &[]);
            let arg = dn(ctx, p);
            ctx.tl("iceOther", &[arg]);
        }
        return Ok(());
    }
    let me = ctx.player()?;
    ctx.host.set_freeze_ticks(me, MAX_FREEZE);
    ctx.tl("ice", &[]);
    Ok(())
}

const TIME_SINCE_REST: &str = "minecraft:custom/minecraft:time_since_rest";

fn cmd_rest(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() && !ctx.is_player() {
        return Err(CmdError::Usage(None));
    }
    if !args.is_empty() && ctx.has("essentials.rest.others") {
        for p in loop_online(ctx, false, true, &args[0])? {
            ctx.host.set_statistic(p, TIME_SINCE_REST, 0);
            ctx.tl_to(Sender::Player(p), "rest", &[]);
            let arg = dn(ctx, p);
            ctx.tl("restOther", &[arg]);
        }
        return Ok(());
    }
    let me = ctx.player()?;
    ctx.host.set_statistic(me, TIME_SINCE_REST, 0);
    ctx.tl("rest", &[]);
    Ok(())
}

fn cmd_suicide(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    ctx.host.set_health(me, 0.0);
    ctx.tl("suicideMessage", &[]);
    let arg = dn(ctx, me);
    broadcast_tl(ctx.ess, ctx.host, Some(me), "suicideSuccess", &[arg]);
    Ok(())
}

// ===== /gamemode ====================================================================================

/// Commandgamemode.matchGameMode: Ok(None) = toggle, Err = not a mode.
fn match_game_mode(s: &str) -> Result<Option<GameMode>, CmdError> {
    let m = s.to_lowercase();
    let is = |a: &str| m == a;
    if is("gmc") || is("egmc") || m.contains("creat") || is("1") || is("c") {
        Ok(Some(GameMode::Creative))
    } else if is("gms") || is("egms") || m.contains("survi") || is("0") || is("s") {
        Ok(Some(GameMode::Survival))
    } else if is("gma") || is("egma") || m.contains("advent") || is("2") || is("a") {
        Ok(Some(GameMode::Adventure))
    } else if is("gmsp") || is("egmsp") || m.contains("spec") || is("3") || is("sp") {
        Ok(Some(GameMode::Spectator))
    } else if is("gmt") || is("egmt") || m.contains("toggle") || m.contains("cycle") || is("t") {
        Ok(None)
    } else {
        Err(CmdError::Usage(None))
    }
}

fn prohibited(ctx: &mut Ctx, to: GameMode) -> bool {
    ctx.is_player() && !ctx.has("essentials.gamemode.all") && !ctx.has(&format!("essentials.gamemode.{}", to.name()))
}

fn set_user_gamemode(ctx: &mut Ctx, mode: Option<GameMode>, player: Uuid) -> CmdResult {
    let Some(mode) = mode else {
        let extra = ctx.ess.tl("gameModeInvalid", &[]);
        return Err(CmdError::Usage(Some(extra)));
    };
    if prohibited(ctx, mode) {
        ctx.tl("cantGamemode", &[Arg::Plain(format!("{mode:?}").to_uppercase())]);
        return Ok(());
    }
    ctx.host.set_game_mode(player, mode);
    let args = [Arg::Plain(ctx.ess.tl(mode.name(), &[])), dn(ctx, player)];
    ctx.tl("gameMode", &args);
    Ok(())
}

fn cmd_gamemode(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let label = ctx.label.clone();
    let Sender::Player(me) = ctx.sender else {
        match args.len() {
            0 => return Err(CmdError::Usage(None)),
            1 => {
                for p in loop_online(ctx, false, true, &args[0])? {
                    let mode = match_game_mode(&label)?;
                    set_user_gamemode(ctx, mode, p)?;
                }
            }
            2 => {
                for p in loop_online(ctx, false, true, &args[1])? {
                    let mode = match_game_mode(&args[0])?;
                    set_user_gamemode(ctx, mode, p)?;
                }
            }
            _ => {}
        }
        return Ok(());
    };
    let mode = if args.is_empty() {
        match_game_mode(&label)?
    } else if args.len() > 1 && args[1].trim().chars().count() > 2 && ctx.has("essentials.gamemode.others") {
        for p in loop_online(ctx, false, true, &args[1])? {
            let mode = match_game_mode(&args[0])?;
            set_user_gamemode(ctx, mode, p)?;
        }
        return Ok(());
    } else {
        match match_game_mode(&args[0]) {
            Ok(m) => m,
            Err(_) if ctx.has("essentials.gamemode.others") => {
                for p in loop_online(ctx, false, true, &args[0])? {
                    let mode = match_game_mode(&label)?;
                    set_user_gamemode(ctx, mode, p)?;
                }
                return Ok(());
            }
            Err(e) => return Err(e),
        }
    };
    let mode = mode.unwrap_or_else(|| match ctx.host.game_mode(me) {
        GameMode::Survival => GameMode::Creative,
        GameMode::Creative => GameMode::Adventure,
        _ => GameMode::Survival,
    });
    if prohibited(ctx, mode) {
        let name = ctx.ess.tl(mode.name(), &[]);
        ctx.tl("cantGamemode", &[Arg::Plain(name)]);
        return Ok(());
    }
    let preserve = ctx.ess.config_bool("gamemode-change-preserve-flying", false) && ctx.has("essentials.fly") && ctx.host.flying(me) && ctx.host.allow_flight(me);
    ctx.host.set_game_mode(me, mode);
    if preserve {
        ctx.host.set_allow_flight(me, true);
        ctx.host.set_flying(me, true);
    }
    let now = ctx.host.game_mode(me);
    let args = [Arg::Plain(ctx.ess.tl(now.name(), &[])), dn(ctx, me)];
    ctx.tl("gameMode", &args);
    Ok(())
}

// ===== /speed =======================================================================================

fn is_fly_mode(s: &str) -> Result<bool, CmdError> {
    if s.contains("fly") || s.eq_ignore_ascii_case("f") {
        Ok(true)
    } else if s.contains("walk") || s.contains("run") || s.eq_ignore_ascii_case("w") || s.eq_ignore_ascii_case("r") {
        Ok(false)
    } else {
        Err(CmdError::Usage(None))
    }
}

fn move_speed(s: &str) -> Result<f32, CmdError> {
    let v: f32 = s.parse().map_err(|_| CmdError::Usage(None))?;
    if !v.is_finite() {
        return Err(CmdError::Usage(None));
    }
    Ok(v.clamp(0.0001, 10.0))
}

fn real_move_speed(ess: &Ess, user_speed: f32, fly: bool, bypass: bool) -> f32 {
    let default = if fly { 0.1f32 } else { 0.2f32 };
    let max = if bypass { 1.0 } else if fly { ess.config_f64("max-fly-speed", 0.8) as f32 } else { ess.config_f64("max-walk-speed", 0.8) as f32 };
    if user_speed < 1.0 { default * user_speed } else { ((user_speed - 1.0) / 9.0) * (max - default) + default }
}

fn fly_perm_check(ctx: &mut Ctx, input: bool) -> bool {
    let can_fly = ctx.has("essentials.speed.fly");
    let can_walk = ctx.has("essentials.speed.walk");
    if (input && can_fly) || (!input && can_walk) || (!can_fly && !can_walk) { input } else { !can_walk }
}

fn apply_speed(ctx: &mut Ctx, player: Uuid, fly: bool, bypass: bool, speed: f32) {
    let real = real_move_speed(ctx.ess, speed, fly, bypass);
    if fly {
        ctx.host.set_fly_speed(player, real);
    } else {
        ctx.host.set_walk_speed(player, real);
    }
    let args = [tl_parsed(ctx, if fly { "flying" } else { "walking" }), Arg::Plain(fmt_num(speed as f64)), dn(ctx, player)];
    ctx.tl("moveSpeed", &args);
}

fn speed_others(ctx: &mut Ctx, fly: bool, bypass: bool, speed: f32, name: &str) -> CmdResult {
    let found = match_players(ctx, name);
    if found.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    for p in found {
        apply_speed(ctx, p, fly, bypass, speed);
    }
    Ok(())
}

fn cmd_speed(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else {
        if args.len() < 3 {
            return Err(CmdError::Usage(None));
        }
        let fly = is_fly_mode(&args[0])?;
        let speed = move_speed(&args[1])?;
        return speed_others(ctx, fly, true, speed, &args[2]);
    };
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let bypass = ctx.has("essentials.speed.bypass");
    let label = ctx.label.to_lowercase();
    let (fly, speed);
    if args.len() == 1 {
        let inferred = if label.contains("fly") || label == "fspeed" || label == "efspeed" {
            true
        } else if label.contains("walk") || label == "wspeed" || label == "ewspeed" {
            false
        } else {
            ctx.host.flying(me)
        };
        fly = fly_perm_check(ctx, inferred);
        speed = move_speed(&args[0])?;
    } else {
        let wanted = is_fly_mode(&args[0])?;
        fly = fly_perm_check(ctx, wanted);
        speed = move_speed(&args[1])?;
        if args.len() > 2 && ctx.has("essentials.speed.others") {
            if args[2].trim().chars().count() < 2 {
                return Err(CmdError::PlayerNotFound);
            }
            return speed_others(ctx, fly, bypass, speed, &args[2]);
        }
    }
    apply_speed(ctx, me, fly, bypass, speed);
    Ok(())
}

// ===== Time (DescParseTickFormat) ===================================================================

const TICK_NAMES: &[(&str, i64)] = &[
    ("sunrise", 23000), ("dawn", 23000), ("daystart", 0), ("day", 0), ("morning", 1000), ("midday", 6000), ("noon", 6000),
    ("afternoon", 9000), ("sunset", 12000), ("dusk", 12000), ("sundown", 12000), ("nightfall", 12000), ("nightstart", 14000),
    ("night", 14000), ("midnight", 18000),
];

fn hm_to_ticks(h: i64, m: i64) -> i64 {
    let mut ret = 18000 + h * 1000 + ((m as f64 / 60.0) * 1000.0) as i64;
    ret %= 24000;
    ret
}

/// DescParseTickFormat.parse: ticks ("1000", "1000t"), 24h ("13:30"), 12h ("6pm", "6:30am") or a name.
pub fn parse_time(desc: &str) -> Option<i64> {
    let d: String = desc.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || *c == ':').collect();
    // ticks
    let digits: String = d.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = &d[digits.len()..];
    if !digits.is_empty() && ["", "t", "ti", "tic", "tick", "ticks"].contains(&rest) {
        return digits.parse::<i64>().ok().map(|v| v % 24000);
    }
    // 24h: two digits, an optional separator, two digits
    let chars: Vec<char> = d.chars().collect();
    let two = |a: usize| chars.get(a).is_some_and(|c| c.is_ascii_digit()) && chars.get(a + 1).is_some_and(|c| c.is_ascii_digit());
    if (chars.len() == 4 && two(0) && two(2)) || (chars.len() == 5 && two(0) && !chars[2].is_ascii_digit() && two(3)) {
        let n: String = chars.iter().filter(|c| c.is_ascii_digit()).collect();
        return Some(hm_to_ticks(n[..2].parse().ok()?, n[2..].parse().ok()?));
    }
    // 12h
    if let Some(body) = d.strip_suffix("pm").or_else(|| d.strip_suffix("am")) {
        let n: String = body.chars().filter(|c| c.is_ascii_digit()).collect();
        let ok_shape = !body.is_empty() && body.chars().next().is_some_and(|c| c.is_ascii_digit()) && n.len() <= 4 && body.chars().filter(|c| !c.is_ascii_digit()).count() <= 1;
        if ok_shape && !n.is_empty() {
            let (mut h, m): (i64, i64) = match n.len() {
                4 => (n[..2].parse().ok()?, n[2..].parse().ok()?),
                3 => (n[..1].parse().ok()?, n[1..].parse().ok()?),
                _ => (n.parse().ok()?, 0),
            };
            if d.ends_with("pm") && h != 12 {
                h += 12;
            }
            if d.ends_with("am") && h == 12 {
                h -= 12;
            }
            return Some(hm_to_ticks(h, m));
        }
    }
    TICK_NAMES.iter().find(|(n, _)| *n == d).map(|(_, t)| *t)
}

fn means_reset(s: &str) -> bool {
    matches!(s, "reset" | "normal" | "default")
}

/// (hours, minutes) on the clock for a tick count (ticksToDate).
fn clock(ticks: i64) -> (i64, i64) {
    let mut t = (ticks - 18000 + 24000).rem_euclid(24000);
    let hours = t / 1000;
    t -= hours * 1000;
    let minutes = (t as f64 / (1000.0 / 60.0)).floor() as i64;
    let dticks = t as f64 - minutes as f64 * (1000.0 / 60.0);
    let seconds = (dticks / (1000.0 / 3600.0)).floor() as i64 + 1;
    // Rounding carries seconds into the minute as Calendar would.
    let total = hours * 3600 + minutes * 60 + seconds;
    ((total / 3600) % 24, (total / 60) % 60)
}

pub fn format_ticks(ticks: i64) -> String {
    format!("{} ticks", ticks % 24000)
}

pub fn format_24(ticks: i64) -> String {
    let (h, m) = clock(ticks);
    format!("{h:02}:{m:02}")
}

pub fn format_12(ticks: i64) -> String {
    let (h, m) = clock(ticks);
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    format!("{h12}:{m:02} {}", if h < 12 { "AM" } else { "PM" })
}

/// DescParseTickFormat.format: the `timeFormat` message (MiniMessage).
pub fn format_time(ess: &Ess, ticks: i64) -> String {
    ess.tl("timeFormat", &[Arg::Plain(format_24(ticks)), Arg::Plain(format_12(ticks)), Arg::Plain(format_ticks(ticks))])
}

fn time_worlds(ctx: &mut Ctx, selector: Option<&str>) -> Result<Vec<String>, CmdError> {
    let mut worlds = match selector {
        None => match ctx.sender {
            Sender::Player(p) => ctx.host.location(p).map(|l| vec![l.world]).unwrap_or_default(),
            Sender::Console => ctx.host.worlds(),
        },
        Some(sel) => match find_world(ctx.host, sel) {
            Some(w) => vec![w],
            None if sel == "*" || sel.eq_ignore_ascii_case("all") => ctx.host.worlds(),
            None => return Err(CmdError::Tl("invalidWorld", vec![])),
        },
    };
    worlds.sort_by_key(|w| world_name(w));
    worlds.dedup();
    Ok(worlds)
}

fn parse_time_arg(s: &str) -> Result<i64, CmdError> {
    let s = if s.parse::<i32>().is_ok() { format!("{s}t") } else { s.to_string() };
    parse_time(&s).ok_or(CmdError::Usage(None))
}

fn cmd_time(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let label = ctx.label.to_lowercase();
    let mut add = false;
    let (tick, worlds);
    if args.is_empty() {
        worlds = time_worlds(ctx, None)?;
        if label.ends_with("day") || label.ends_with("night") {
            tick = parse_time(&label.replace('e', "")).unwrap_or(0);
        } else {
            if worlds.len() == 1 {
                let t = ctx.host.world_time(&worlds[0]);
                let text = format_time(ctx.ess, t);
                ctx.send_mini(&text);
                return Ok(());
            }
            for w in worlds {
                let t = ctx.host.world_time(&w);
                let text = format_time(ctx.ess, t);
                ctx.tl("timeWorldCurrent", &[Arg::Plain(world_name(&w)), Arg::Parsed(text)]);
            }
            return Ok(());
        }
    } else if args.len() == 1 {
        worlds = time_worlds(ctx, None)?;
        tick = parse_time_arg(&args[0])?;
    } else if args[0].eq_ignore_ascii_case("set") || args[0].eq_ignore_ascii_case("add") {
        add = args[0].eq_ignore_ascii_case("add");
        tick = parse_time_arg(&args[1])?;
        worlds = time_worlds(ctx, args.get(2).map(String::as_str))?;
    } else {
        tick = parse_time_arg(&args[0])?;
        worlds = time_worlds(ctx, Some(&args[1]))?;
    }
    if !ctx.has("essentials.time.set") {
        return Err(CmdError::Tl("timeSetPermission", vec![]));
    }
    let per_world = ctx.ess.config_bool("world-time-permissions", false);
    for w in &worlds {
        let all = !per_world || ctx.has("essentials.time.world.all");
        let name = world_name(w).to_lowercase().split_whitespace().collect::<Vec<_>>().join("_");
        if !all && !ctx.has(&format!("essentials.time.world.{name}")) {
            let here = ctx.sender.player().and_then(|p| ctx.host.location(p)).map(|l| world_name(&l.world)).unwrap_or_default();
            return Err(CmdError::Tl("timeSetWorldPermission", vec![Arg::Plain(here)]));
        }
    }
    let mut names = Vec::new();
    for w in &worlds {
        // Relative ptimes keep the time their players see.
        let mut keep = Vec::new();
        for p in ctx.host.online() {
            if ctx.host.location(p).is_some_and(|l| &l.world == w) {
                if let Some((off, true)) = ctx.ess.state::<WorldState>().ptime.get(&p).copied() {
                    if off != 0 {
                        let wt = ctx.host.world_time(w);
                        keep.push((p, wt + off));
                    }
                }
            }
        }
        let mut time = ctx.host.world_time(w);
        if !add {
            time -= time % 24000;
        }
        ctx.host.set_world_time(w, time + if add { 0 } else { 24000 } + tick);
        let now = ctx.host.world_time(w);
        for (p, seen) in keep {
            set_ptime(ctx.ess, ctx.host, p, Some((seen - now, true)));
        }
        names.push(world_name(w));
    }
    ctx.tl(if add { "timeWorldAdd" } else { "timeWorldSet" }, &[Arg::Plain(format_ticks(tick)), Arg::Plain(names.join(", "))]);
    Ok(())
}

fn set_ptime(ess: &mut Ess, host: &mut dyn Host, player: Uuid, time: Option<(i64, bool)>) {
    match time {
        Some(t) => ess.state::<WorldState>().ptime.insert(player, t),
        None => ess.state::<WorldState>().ptime.remove(&player),
    };
    host.set_player_time(player, time);
}

/// Player.getPlayerTime: the world time plus a relative offset, or the fixed time.
fn player_time(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> (i64, Option<bool>) {
    let world = host.location(player).map(|l| l.world).unwrap_or_default();
    let wt = host.world_time(&world);
    match ess.state::<WorldState>().ptime.get(&player).copied() {
        Some((off, true)) => (wt + off, Some(true)),
        Some((t, false)) => (t, Some(false)),
        None => (wt, None),
    }
}

fn self_selector(ctx: &mut Ctx) -> String {
    match ctx.sender {
        Sender::Player(p) => ctx.name_of(p),
        Sender::Console => "*".into(),
    }
}

const GET_ALIASES: &[&str] = &["get", "list", "show", "display"];

fn ptime_get(ctx: &mut Ctx, player: Uuid) {
    let name = ctx.name_of(player);
    match player_time(ctx.ess, ctx.host, player) {
        (_, None) => ctx.tl("pTimeNormal", &[Arg::Plain(name)]),
        (t, Some(relative)) => {
            let text = format_time(ctx.ess, t);
            ctx.tl(if relative { "pTimeCurrent" } else { "pTimeCurrentFixed" }, &[Arg::Plain(name), Arg::Parsed(text)]);
        }
    }
}

fn cmd_ptime(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() || GET_ALIASES.contains(&args[0].to_lowercase().as_str()) {
        if args.len() > 1 {
            if args[1] == "*" || args[1] == "**" {
                ctx.tl("pTimePlayers", &[]);
            }
            for p in loop_online(ctx, false, true, &args[1])? {
                ptime_get(ctx, p);
            }
            return Ok(());
        }
        if let Sender::Player(me) = ctx.sender {
            ptime_get(ctx, me);
            return Ok(());
        }
        if args.len() == 1 {
            return Err(CmdError::Usage(None));
        }
        let all = online_sorted(ctx.host);
        if all.len() > 1 {
            ctx.tl("pTimePlayers", &[]);
        }
        for p in all {
            ptime_get(ctx, p);
        }
        return Ok(());
    }
    let me_sel = self_selector(ctx);
    if args.len() > 1 && !ctx.has("essentials.ptime.others") && !args[1].eq_ignore_ascii_case(&me_sel) {
        ctx.tl("pTimeOthersPermission", &[]);
        return Ok(());
    }
    let mut time = args[0].as_str();
    let fixed = time.starts_with('@');
    if fixed {
        time = &time[1..];
    }
    let ticks = if means_reset(time) { None } else { Some(parse_time(time).ok_or(CmdError::Usage(None))?) };
    let target = args.get(1).cloned().unwrap_or(me_sel);
    let mut names = Vec::new();
    for p in loop_online(ctx, false, true, &target)? {
        match ticks {
            None => set_ptime(ctx.ess, ctx.host, p, None),
            Some(t) => {
                let world = ctx.host.location(p).map(|l| l.world).unwrap_or_default();
                let (mut time, _) = player_time(ctx.ess, ctx.host, p);
                time -= time % 24000;
                time += 24000 + t;
                if !fixed {
                    time -= ctx.host.world_time(&world);
                }
                set_ptime(ctx.ess, ctx.host, p, Some((time, !fixed)));
            }
        }
        names.push(ctx.name_of(p));
    }
    let joined = Arg::Plain(names.join(", "));
    match ticks {
        None => ctx.tl("pTimeReset", &[joined]),
        Some(t) => {
            let text = format_time(ctx.ess, t);
            ctx.tl(if fixed { "pTimeSetFixed" } else { "pTimeSet" }, &[Arg::Parsed(text), joined]);
        }
    }
    Ok(())
}

// ===== Weather ======================================================================================

fn pweather_get(ctx: &mut Ctx, player: Uuid) {
    let name = ctx.name_of(player);
    match ctx.ess.state::<WorldState>().pweather.get(&player).copied() {
        None => ctx.tl("pWeatherNormal", &[Arg::Plain(name)]),
        Some(down) => ctx.tl("pWeatherCurrent", &[Arg::Plain(name), Arg::Plain(if down { "downfall" } else { "clear" }.into())]),
    }
}

fn cmd_pweather(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() || GET_ALIASES.contains(&args[0].to_lowercase().as_str()) {
        if args.len() > 1 {
            if args[1] == "*" || args[1] == "**" {
                ctx.tl("pWeatherPlayers", &[]);
            }
            for p in loop_online(ctx, false, true, &args[1])? {
                pweather_get(ctx, p);
            }
            return Ok(());
        }
        if let Sender::Player(me) = ctx.sender {
            pweather_get(ctx, me);
            return Ok(());
        }
        if args.len() == 1 {
            return Err(CmdError::Usage(None));
        }
        let all = online_sorted(ctx.host);
        if all.len() > 1 {
            ctx.tl("pWeatherPlayers", &[]);
        }
        for p in all {
            pweather_get(ctx, p);
        }
        return Ok(());
    }
    let me_sel = self_selector(ctx);
    if args.len() > 1 && !ctx.has("essentials.pweather.others") && !args[1].eq_ignore_ascii_case(&me_sel) {
        ctx.tl("pWeatherOthersPermission", &[]);
        return Ok(());
    }
    let weather = args[0].to_lowercase();
    let downfall = match weather.as_str() {
        "sun" | "clear" => Some(false),
        "storm" | "thunder" => Some(true),
        "reset" => None,
        _ => {
            let extra = ctx.ess.tl("pWeatherInvalidAlias", &[]);
            return Err(CmdError::Usage(Some(extra)));
        }
    };
    let target = args.get(1).cloned().unwrap_or(me_sel);
    let mut names = Vec::new();
    for p in loop_online(ctx, false, true, &target)? {
        match downfall {
            Some(d) => ctx.ess.state::<WorldState>().pweather.insert(p, d),
            None => ctx.ess.state::<WorldState>().pweather.remove(&p),
        };
        ctx.host.set_player_weather(p, downfall);
        names.push(ctx.name_of(p));
    }
    let joined = Arg::Plain(names.join(", "));
    if downfall.is_none() {
        ctx.tl("pWeatherReset", &[joined]);
    } else {
        ctx.tl("pWeatherSet", &[Arg::Plain(weather), joined]);
    }
    Ok(())
}

fn set_storm(ctx: &mut Ctx, world: &str, storm: bool, seconds: Option<&str>) -> CmdResult {
    let duration = match seconds {
        Some(s) => Some((parse_int(s)? * 20) as i32),
        None => None,
    };
    let (_, thunder) = ctx.host.weather(world);
    ctx.host.set_weather(world, storm, thunder, duration);
    let name = Arg::Plain(world_name(world));
    match seconds {
        Some(s) => ctx.tl(if storm { "weatherStormFor" } else { "weatherSunFor" }, &[name, Arg::Plain(s.to_string())]),
        None => ctx.tl(if storm { "weatherStorm" } else { "weatherSun" }, &[name]),
    }
    Ok(())
}

fn cmd_weather(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else {
        if args.len() < 2 {
            return Err(CmdError::Msg(format!("When running from console, usage is: /{} <world> <storm/sun> [duration]", ctx.label)));
        }
        let storm = args[1].eq_ignore_ascii_case("storm");
        let Some(world) = find_world(ctx.host, &args[0]) else {
            return Err(CmdError::Tl("weatherInvalidWorld", vec![Arg::Plain(args[0].clone())]));
        };
        return set_storm(ctx, &world, storm, args.get(2).map(String::as_str));
    };
    let label = ctx.label.to_lowercase();
    let storm = if args.is_empty() {
        if label.ends_with("sun") {
            false
        } else if label.ends_with("storm") || label.ends_with("rain") {
            true
        } else {
            return Err(CmdError::Usage(None));
        }
    } else {
        args[0].eq_ignore_ascii_case("storm")
    };
    let world = ctx.host.location(me).map(|l| l.world).unwrap_or_default();
    set_storm(ctx, &world, storm, args.get(1).map(String::as_str))
}

fn cmd_thunder(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let world = ctx.host.location(me).map(|l| l.world).unwrap_or_default();
    let on = args[0].eq_ignore_ascii_case("true");
    let (rain, _) = ctx.host.weather(&world);
    if args.len() == 1 {
        ctx.host.set_weather(&world, rain, on, None);
        let a = enable_disable(ctx, on);
        ctx.tl("thunder", &[a]);
        return Ok(());
    }
    let secs = parse_int(&args[1])?;
    ctx.host.set_weather(&world, rain, on, Some((secs * 20) as i32));
    let a = enable_disable(ctx, on);
    ctx.tl("thunderDuration", &[a, Arg::Plain(secs.to_string())]);
    Ok(())
}

// ===== /lightning ===================================================================================

fn cmd_lightning(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() || !ctx.has("essentials.lightning.others") {
        if let Sender::Player(me) = ctx.sender {
            let at = target(ctx, me, 600.0);
            ctx.host.strike_lightning(&at, false);
            return Ok(());
        }
        return Err(CmdError::Usage(None));
    }
    let power = args.get(1).and_then(|p| p.parse::<i32>().ok()).unwrap_or(5);
    for p in loop_online(ctx, false, true, &args[0])? {
        let arg = dn(ctx, p);
        ctx.tl("lightningUse", &[arg]);
        if let Some(at) = ctx.host.location(p) {
            ctx.host.strike_lightning(&at, true);
        }
        if !is_god(ctx.ess, ctx.host, p) {
            let name = ctx.name_of(p);
            ctx.host.console_command(&format!("damage {name} {power} minecraft:lightning_bolt"));
        }
        if ctx.ess.config_bool("warn-on-smite", true) {
            ctx.tl_to(Sender::Player(p), "lightningSmited", &[]);
        }
    }
    Ok(())
}

// ===== /tree /bigtree /break ========================================================================

/// Bukkit TreeType -> the vanilla configured feature CraftBukkit grows.
const TREES: &[(&str, &str)] = &[
    ("TREE", "oak"), ("BIG_TREE", "fancy_oak"), ("REDWOOD", "spruce"), ("TALL_REDWOOD", "pine"), ("BIRCH", "birch"),
    ("JUNGLE", "mega_jungle_tree"), ("SMALL_JUNGLE", "jungle_tree_no_vine"), ("COCOA_TREE", "jungle_tree"), ("JUNGLE_BUSH", "jungle_bush"),
    ("RED_MUSHROOM", "huge_red_mushroom"), ("BROWN_MUSHROOM", "huge_brown_mushroom"), ("SWAMP", "swamp_oak"), ("ACACIA", "acacia"),
    ("DARK_OAK", "dark_oak"), ("MEGA_REDWOOD", "mega_spruce"), ("MEGA_PINE", "mega_pine"), ("TALL_BIRCH", "super_birch_bees_0002"),
    ("CHORUS_PLANT", "chorus_plant"), ("CRIMSON_FUNGUS", "crimson_fungus_planted"), ("WARPED_FUNGUS", "warped_fungus_planted"),
    ("AZALEA", "azalea_tree"), ("MANGROVE", "mangrove"), ("TALL_MANGROVE", "tall_mangrove"), ("CHERRY", "cherry"), ("PALE_OAK", "pale_oak"),
    ("PALE_OAK_CREAKING", "pale_oak_creaking"),
];

fn grow_tree(ctx: &mut Ctx, me: Uuid, feature: &str, failure: &'static str) -> Result<bool, CmdError> {
    let range = ctx.ess.config_f64("tree-command-range-limit", 300.0);
    let mut at = target(ctx, me, range);
    at.y += 1.0;
    let [x, y, z] = at.block();
    let state = ctx.host.block(&at.world, [x, y, z]).unwrap_or_else(|| "minecraft:air".into());
    if !is_hollow(&state) {
        return Err(CmdError::Tl(failure, vec![]));
    }
    let (ok, _) = ctx.host.console_command(&format!("execute in {} run place feature minecraft:{feature} {x} {y} {z}", at.world));
    Ok(ok)
}

fn cmd_tree(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let Some(arg) = args.first() else { return Err(CmdError::Usage(None)) };
    let mut tree = TREES.iter().find(|(n, _)| n.replace('_', "").eq_ignore_ascii_case(arg)).map(|(_, f)| *f);
    if arg.eq_ignore_ascii_case("jungle") {
        tree = Some("jungle_tree_no_vine");
    }
    let Some(feature) = tree else { return Err(CmdError::Usage(None)) };
    if grow_tree(ctx, me, feature, "treeFailure")? {
        ctx.tl("treeSpawned", &[]);
    } else {
        ctx.tl("treeFailure", &[]);
    }
    Ok(())
}

fn cmd_bigtree(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let feature = match args.first().map(|a| a.to_lowercase()).as_deref() {
        Some("redwood") => "pine",
        Some("tree") => "fancy_oak",
        Some("jungle") => "mega_jungle_tree",
        Some("darkoak") => "dark_oak",
        _ => return Err(CmdError::Usage(None)),
    };
    if grow_tree(ctx, me, feature, "bigTreeFailure")? {
        ctx.tl("bigTreeSuccess", &[]);
        Ok(())
    } else {
        Err(CmdError::Tl("bigTreeFailure", vec![]))
    }
}

fn cmd_break(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let Some(pos) = ctx.host.target_block(me, 20.0) else { return Err(CmdError::Quiet) };
    let world = ctx.host.location(me).map(|l| l.world).unwrap_or_default();
    let state = ctx.host.block(&world, pos).unwrap_or_else(|| "minecraft:air".into());
    let id = block_id(&state);
    if matches!(id, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") {
        return Err(CmdError::Quiet);
    }
    if id == "minecraft:bedrock" && !ctx.has("essentials.break.bedrock") {
        return Err(CmdError::Tl("noBreakBedrock", vec![]));
    }
    if ctx.ess.on_block_break(ctx.host, me, &world, pos, &state) == Allow::No {
        return Err(CmdError::Quiet);
    }
    ctx.host.set_block(&world, pos, "minecraft:air");
    Ok(())
}

// ===== Mobs (Mob.java, SpawnMob.java, MobData.java) =================================================

/// (EssentialsX name, entity id without namespace, plural suffix).
pub const MOBS: &[(&str, &str, &str)] = &[
    ("Chicken", "chicken", "s"), ("Cow", "cow", "s"), ("Creeper", "creeper", "s"), ("Ghast", "ghast", "s"), ("Giant", "giant", "s"),
    ("Horse", "horse", "s"), ("Pig", "pig", "s"), ("PigZombie", "zombified_piglin", "s"), ("ZombifiedPiglin", "zombified_piglin", "s"),
    ("Sheep", "sheep", ""), ("Skeleton", "skeleton", "s"), ("Slime", "slime", "s"), ("Spider", "spider", "s"), ("Squid", "squid", "s"),
    ("Zombie", "zombie", "s"), ("Wolf", "wolf", ""), ("CaveSpider", "cave_spider", "s"), ("Enderman", "enderman", ""),
    ("Silverfish", "silverfish", ""), ("EnderDragon", "ender_dragon", "s"), ("Villager", "villager", "s"), ("Blaze", "blaze", "s"),
    ("MushroomCow", "mooshroom", "s"), ("MagmaCube", "magma_cube", "s"), ("Snowman", "snow_golem", ""), ("Ocelot", "ocelot", "s"),
    ("IronGolem", "iron_golem", "s"), ("Wither", "wither", "s"), ("Bat", "bat", "s"), ("Witch", "witch", "s"), ("Boat", "oak_boat", "s"),
    ("AcaciaBoat", "acacia_boat", "s"), ("DarkOakBoat", "dark_oak_boat", "s"), ("BirchBoat", "birch_boat", "s"),
    ("JungleBoat", "jungle_boat", "s"), ("SpruceBoat", "spruce_boat", "s"), ("MangroveBoat", "mangrove_boat", "s"),
    ("CherryBoat", "cherry_boat", "s"), ("BambooRaft", "bamboo_raft", "s"), ("PaleOakBoat", "pale_oak_boat", "s"),
    ("Minecart", "minecart", "s"), ("ChestMinecart", "chest_minecart", "s"), ("FurnaceMinecart", "furnace_minecart", "s"),
    ("TNTMinecart", "tnt_minecart", "s"), ("HopperMinecart", "hopper_minecart", "s"), ("SpawnerMinecart", "spawner_minecart", "s"),
    ("EnderCrystal", "end_crystal", "s"), ("ExperienceOrb", "experience_orb", "s"), ("ArmorStand", "armor_stand", "s"),
    ("Endermite", "endermite", "s"), ("Guardian", "guardian", "s"), ("ElderGuardian", "elder_guardian", "s"), ("Rabbit", "rabbit", "s"),
    ("Shulker", "shulker", "s"), ("PolarBear", "polar_bear", "s"), ("WitherSkeleton", "wither_skeleton", "s"),
    ("StraySkeleton", "stray", "s"), ("ZombieVillager", "zombie_villager", "s"), ("SkeletonHorse", "skeleton_horse", "s"),
    ("ZombieHorse", "zombie_horse", "s"), ("Donkey", "donkey", "s"), ("Mule", "mule", "s"), ("Evoker", "evoker", "s"), ("Vex", "vex", "s"),
    ("Vindicator", "vindicator", "s"), ("Llama", "llama", "s"), ("Husk", "husk", "s"), ("Illusioner", "illusioner", "s"),
    ("Parrot", "parrot", "s"), ("Turtle", "turtle", "s"), ("Phantom", "phantom", "s"), ("Cod", "cod", ""), ("Salmon", "salmon", ""),
    ("Pufferfish", "pufferfish", ""), ("TropicalFish", "tropical_fish", ""), ("Drowned", "drowned", "s"), ("Dolphin", "dolphin", "s"),
    ("Cat", "cat", "s"), ("Fox", "fox", "es"), ("Panda", "panda", "s"), ("Pillager", "pillager", "s"), ("Ravager", "ravager", "s"),
    ("TraderLlama", "trader_llama", "s"), ("WanderingTrader", "wandering_trader", "s"), ("Bee", "bee", "s"), ("Stray", "stray", "s"),
    ("Hoglin", "hoglin", "s"), ("Piglin", "piglin", "s"), ("Strider", "strider", "s"), ("Zoglin", "zoglin", "s"),
    ("PiglinBrute", "piglin_brute", "s"), ("Axolotl", "axolotl", "s"), ("Goat", "goat", "s"), ("GlowSquid", "glow_squid", "s"),
    ("Allay", "allay", "s"), ("Frog", "frog", "s"), ("Tadpole", "tadpole", "s"), ("Warden", "warden", "s"), ("ChestBoat", "oak_chest_boat", "s"),
    ("AcaciaChestBoat", "acacia_chest_boat", "s"), ("DarkOakChestBoat", "dark_oak_chest_boat", "s"), ("BirchChestBoat", "birch_chest_boat", "s"),
    ("JungleChestBoat", "jungle_chest_boat", "s"), ("SpruceChestBoat", "spruce_chest_boat", "s"),
    ("MangroveChestBoat", "mangrove_chest_boat", "s"), ("CherryChestBoat", "cherry_chest_boat", "s"),
    ("BambooChestRaft", "bamboo_chest_raft", "s"), ("PaleOakChestBoat", "pale_oak_chest_boat", "s"), ("Camel", "camel", "s"),
    ("Sniffer", "sniffer", "s"), ("Armadillo", "armadillo", "s"), ("Breeze", "breeze", "s"), ("Bogged", "bogged", "s"),
    ("Creaking", "creaking", "s"), ("HappyGhast", "happy_ghast", "s"), ("CopperGolem", "copper_golem", "s"), ("CamelHusk", "camel_husk", "s"),
    ("Nautilus", "nautilus", "s"), ("ZombieNautilus", "zombie_nautilus", "s"), ("Parched", "parched", "s"),
    ("SulfurCube", "sulfur_cube", "s"), ("Cushion", "cushion", "s"),
];

/// Mob.fromName (case-insensitive EssentialsX name).
pub fn mob(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    MOBS.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(name)).copied()
}

fn mob_list() -> String {
    let mut names: Vec<String> = MOBS.iter().map(|(n, _, _)| n.to_lowercase()).collect();
    names.sort();
    names.dedup();
    names.join(", ")
}

const DYES: &[&str] = &[
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black",
];

/// MobData: the SNBT parts a data string (`baby`, `red`, `saddle`...) gives an entity, best effort.
fn mob_data(kind: &str, data: &str, owner: Option<Uuid>) -> Vec<String> {
    let mut out = Vec::new();
    let d = data.to_lowercase();
    let zombie_like = matches!(kind, "zombie" | "husk" | "drowned" | "zombie_villager" | "zombified_piglin");
    let words: Vec<&str> = d.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let has = |w: &str| d.contains(w);
    if zombie_like || matches!(kind, "piglin" | "zoglin") {
        out.push(if has("baby") { "IsBaby:1b".into() } else { "IsBaby:0b".into() });
    } else if has("baby") || has("kitten") || has("piglet") || has("puppy") || has("chick") || has("colt") || has("lamb") || has("calf") || has("child") {
        out.push("Age:-24000".into());
    } else if !data.is_empty() {
        out.push("Age:0".into());
    }
    if (has("tame") || has("tamed")) && owner.is_some() {
        out.push(format!("Owner:{}", uuid_ints(owner.unwrap_or(0))));
        if kind == "horse" || kind == "donkey" || kind == "mule" || kind == "llama" {
            out.push("Tame:1b".into());
        }
    }
    if matches!(kind, "sheep" | "shulker") {
        if let Some(i) = DYES.iter().position(|c| words.contains(&c.replace('_', "").as_str()) || words.contains(c)) {
            out.push(format!("Color:{i}b"));
        }
    }
    if kind == "wolf" {
        if let Some(i) = DYES.iter().position(|c| words.contains(c)) {
            out.push(format!("CollarColor:{i}b"));
        }
        if has("angry") || has("rabid") {
            out.push("AngerTime:2147483647".into());
        }
    }
    if has("saddle") && matches!(kind, "pig" | "horse" | "strider" | "camel" | "donkey" | "mule") {
        out.push("equipment:{saddle:{id:\"minecraft:saddle\",count:1}}".into());
    }
    if kind == "creeper" && (has("powered") || has("electric") || has("charged")) {
        out.push("powered:1b".into());
    }
    if kind == "slime" || kind == "magma_cube" {
        if let Some(n) = words.iter().find_map(|w| w.parse::<i32>().ok()) {
            out.push(format!("Size:{}", (n.clamp(1, 100)) - 1));
        }
    }
    if kind == "experience_orb" {
        if let Some(n) = words.iter().find_map(|w| w.parse::<i32>().ok()) {
            out.push(format!("Value:{}s", n.clamp(1, i16::MAX as i32)));
        }
    }
    if kind == "villager" || kind == "zombie_villager" {
        let profession = ["armorer", "butcher", "cartographer", "cleric", "farmer", "fisherman", "fletcher", "leatherworker", "librarian", "mason", "nitwit", "shepherd", "toolsmith", "weaponsmith"]
            .into_iter()
            .find(|p| words.contains(p));
        let biome = ["desert", "jungle", "plains", "savanna", "snowy", "swamp", "taiga"].into_iter().find(|b| words.contains(b));
        if profession.is_some() || biome.is_some() {
            out.push(format!(
                "VillagerData:{{profession:\"minecraft:{}\",type:\"minecraft:{}\",level:1}}",
                profession.unwrap_or("none"),
                biome.unwrap_or("plains")
            ));
        }
    }
    if kind == "cat" {
        let cats = [("siamese", "siamese"), ("white", "white"), ("red", "red"), ("orange", "red"), ("tabby", "tabby"), ("black", "black"), ("tuxedo", "black"), ("britishshorthair", "british_shorthair"), ("calico", "calico"), ("persian", "persian"), ("ragdoll", "ragdoll"), ("jellie", "jellie"), ("allblack", "all_black")];
        if let Some((_, v)) = cats.iter().find(|(w, _)| words.contains(w)) {
            out.push(format!("variant:\"minecraft:{v}\""));
        }
    }
    if kind == "parrot" {
        let parrots = [("red", 0), ("blue", 1), ("green", 2), ("cyan", 3), ("gray", 4)];
        if let Some((_, v)) = parrots.iter().find(|(w, _)| words.contains(w)) {
            out.push(format!("Variant:{v}"));
        }
    }
    if kind == "horse" {
        let colors = [("white", 0), ("creamy", 1), ("flaxen", 1), ("chestnut", 2), ("liver", 2), ("bay", 3), ("brown", 3), ("black", 4), ("gray", 5), ("dapple", 5), ("buckskin", 6), ("darkbrown", 6), ("dark", 6), ("dbrown", 6)];
        let styles = [("socks", 1), ("blaze", 1), ("milky", 2), ("paint", 2), ("splotchy", 2), ("leopard", 3), ("appaloosa", 3), ("polka", 4), ("sooty", 4)];
        let c = colors.iter().find(|(w, _)| words.contains(w)).map(|(_, v)| *v);
        let s = styles.iter().find(|(w, _)| words.contains(w)).map(|(_, v)| *v);
        if c.is_some() || s.is_some() {
            out.push(format!("Variant:{}", c.unwrap_or(0) + s.unwrap_or(0) * 256));
        }
        for (w, item) in [("goldarmor", "golden_horse_armor"), ("diamondarmor", "diamond_horse_armor"), ("netheritearmor", "netherite_horse_armor")] {
            if words.contains(&w) {
                out.push(format!("equipment:{{body:{{id:\"minecraft:{item}\",count:1}}}}"));
            }
        }
    }
    if kind == "zombie" || kind == "skeleton" {
        let swords = [("netheritesword", "netherite_sword"), ("diamondsword", "diamond_sword"), ("goldsword", "golden_sword"), ("ironsword", "iron_sword"), ("stonesword", "stone_sword"), ("sword", "stone_sword"), ("bow", "bow")];
        if let Some((_, item)) = swords.iter().find(|(w, _)| words.contains(w)) {
            out.push(format!("equipment:{{mainhand:{{id:\"minecraft:{item}\",count:1}}}}"));
        }
        if has("armor") || has("armour") {
            let tier = if has("noarmor") || has("noarmour") {
                None
            } else {
                ["netherite", "copper", "diamond", "gold", "leather", "iron"].into_iter().find(|t| has(t))
            };
            if let Some(t) = tier {
                let t = if t == "gold" { "golden" } else { t };
                out.push(format!(
                    "equipment:{{feet:{{id:\"minecraft:{t}_boots\",count:1}},legs:{{id:\"minecraft:{t}_leggings\",count:1}},chest:{{id:\"minecraft:{t}_chestplate\",count:1}},head:{{id:\"minecraft:{t}_helmet\",count:1}}}},drop_chances:{{feet:0f,legs:0f,chest:0f,head:0f}}"
                ));
            }
        }
    }
    out
}

/// SpawnMob.defaultMobData.
fn default_mob_data(kind: &str) -> Vec<String> {
    match kind {
        "skeleton" => vec!["equipment:{mainhand:{id:\"minecraft:bow\",count:1}},drop_chances:{mainhand:0.1f}".into()],
        "zombified_piglin" => vec!["equipment:{mainhand:{id:\"minecraft:golden_sword\",count:1}},drop_chances:{mainhand:0.1f}".into()],
        "horse" => vec!["attributes:[{id:\"minecraft:jump_strength\",base:1.2d}]".into()],
        _ => Vec::new(),
    }
}

/// One mob (and its mounts) as an SNBT compound without the id of the bottom one.
fn mob_snbt(parts: &[(&str, Option<String>)], owner: Option<Uuid>) -> String {
    let mut tags = Vec::new();
    for (i, (kind, data)) in parts.iter().enumerate().rev() {
        let mut fields = default_mob_data(kind);
        if let Some(d) = data {
            fields.extend(mob_data(kind, d, owner));
        }
        if let Some(inner) = tags.pop() {
            fields.push(format!("Passengers:[{{id:\"minecraft:{}\"{}}}]", parts[i + 1].0, inner));
        }
        let joined = fields.join(",");
        tags.push(if joined.is_empty() { String::new() } else { format!(",{joined}") });
    }
    let body = tags.pop().unwrap_or_default();
    format!("{{{}}}", body.trim_start_matches(','))
}

fn cmd_spawnmob(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let usage_mobs = |ctx: &Ctx| CmdError::Usage(Some(ctx.ess.tl("mobsAvailable", &[Arg::Plain(mob_list())])));
    let is_player = ctx.is_player();
    if args.is_empty() || (!is_player && args.len() < 3) {
        return Err(usage_mobs(ctx));
    }
    let pieces: Vec<&str> = args[0].split(',').collect();
    let names: Vec<String> = pieces.iter().map(|p| p.split(':').next().unwrap_or("").to_string()).collect();
    let datas: Vec<Option<String>> = pieces.iter().map(|p| p.find(':').map(|i| p[i + 1..].split(':').next().unwrap_or("").to_string())).collect();
    let mut count = if args.len() >= 2 { parse_int(&args[1])? as i32 } else { 1 };
    if is_player && names.len() > 1 && !ctx.has("essentials.spawnmob.stack") {
        return Err(CmdError::Tl("cannotStackMob", vec![]));
    }
    let (target_player, at) = if args.len() >= 3 {
        let p = get_player(ctx, args, 2)?;
        (p, ctx.host.location(p).unwrap_or_else(|| Loc::new("minecraft:overworld", 0.0, 64.0, 0.0)))
    } else {
        let me = ctx.player()?;
        (me, target(ctx, me, 300.0))
    };
    let at = safe_destination(ctx.host, &at);
    let mut kinds = Vec::new();
    for n in &names {
        let Some((name, kind, _)) = mob(n) else { return Err(CmdError::Tl("invalidMob", vec![])) };
        if ctx.ess.config_bool(&format!("protect.prevent.spawn.{kind}"), false) {
            return Err(CmdError::Tl("disabledToSpawnMob", vec![]));
        }
        if is_player && !ctx.has(&format!("essentials.spawnmob.{}", name.to_lowercase())) {
            return Err(CmdError::Tl("noPermToSpawnMob", vec![]));
        }
        kinds.push(kind);
    }
    let limit = ctx.ess.config_i64("spawnmob-limit", 10).max(1) as usize;
    let mut effective = limit / kinds.len();
    if effective < 1 {
        effective = 1;
        kinds.truncate(limit);
    }
    if count as usize > effective {
        count = effective as i32;
        ctx.tl("mobSpawnLimit", &[]);
    }
    let parts: Vec<(&str, Option<String>)> = kinds.iter().zip(datas.iter()).map(|(k, d)| (*k, d.clone())).collect();
    let snbt = mob_snbt(&parts, Some(target_player));
    for _ in 0..count.max(0) {
        let data = if snbt == "{}" { None } else { Some(snbt.as_str()) };
        if !ctx.host.spawn_entity(&format!("minecraft:{}", kinds[0]), &at, data) {
            return Err(CmdError::Tl("unableToSpawnMob", vec![]));
        }
    }
    let (first, _, suffix) = mob(&names[0]).unwrap_or(("", "", "s"));
    let spawned = ctx.ess.tl("spawned", &[]);
    let line = format!("{} {}{} {}", count.max(0) as usize * kinds.len(), first.to_lowercase(), suffix, spawned);
    ctx.send_mini(&ess_text::escape_tags(&line));
    Ok(())
}

fn cmd_spawner(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() || args[0].chars().count() < 2 {
        return Err(CmdError::Usage(Some(ctx.ess.tl("mobsAvailable", &[Arg::Plain(mob_list())]))));
    }
    let at = target(ctx, me, 300.0);
    let pos = at.block();
    let state = ctx.host.block(&at.world, pos).unwrap_or_default();
    if block_id(&state) != "minecraft:spawner" {
        return Err(CmdError::Tl("mobSpawnTarget", vec![]));
    }
    let Some((name, kind, _)) = mob(&args[0]) else { return Err(CmdError::Tl("invalidMob", vec![])) };
    if ctx.ess.config_bool(&format!("protect.prevent.spawn.{kind}"), false) {
        return Err(CmdError::Tl("disabledToSpawnMob", vec![]));
    }
    if !ctx.has(&format!("essentials.spawner.{}", name.to_lowercase())) {
        return Err(CmdError::Tl("noPermToSpawnMob", vec![]));
    }
    let mut delay = 0;
    if let Some(d) = args.get(1).and_then(|d| d.parse::<i32>().ok()) {
        if ctx.has("essentials.spawner.delay") {
            delay = d;
        }
    }
    let delays = if delay > 0 { format!("MinSpawnDelay:{delay}s,MaxSpawnDelay:{delay}s,Delay:{delay}s") } else { "Delay:0s".into() };
    let line = format!(
        "execute in {} run data merge block {} {} {} {{SpawnData:{{entity:{{id:\"minecraft:{kind}\"}}}},SpawnPotentials:[],{delays}}}",
        at.world, pos[0], pos[1], pos[2]
    );
    let (ok, _) = ctx.host.console_command(&line);
    if !ok {
        return Err(CmdError::Tl("mobSpawnError", vec![]));
    }
    ctx.tl("setSpawner", &[Arg::Plain(name.to_string())]);
    Ok(())
}

// ----- /remove --------------------------------------------------------------------------------------

const MONSTERS: &[&str] = &[
    "zombie", "husk", "drowned", "zombie_villager", "zombified_piglin", "skeleton", "stray", "wither_skeleton", "bogged", "parched", "creeper",
    "spider", "cave_spider", "enderman", "endermite", "silverfish", "blaze", "witch", "guardian", "elder_guardian", "vex", "vindicator", "evoker",
    "pillager", "ravager", "illusioner", "piglin", "piglin_brute", "zoglin", "warden", "breeze", "creaking", "giant", "wither", "ender_dragon",
    "ghast", "phantom", "slime", "magma_cube", "zombie_nautilus",
];
const ANIMALS: &[&str] = &[
    "cow", "pig", "sheep", "chicken", "horse", "donkey", "mule", "llama", "trader_llama", "wolf", "cat", "ocelot", "parrot", "rabbit", "polar_bear",
    "turtle", "fox", "panda", "bee", "hoglin", "strider", "goat", "axolotl", "frog", "camel", "camel_husk", "sniffer", "armadillo", "mooshroom",
    "skeleton_horse", "zombie_horse", "villager", "wandering_trader", "snow_golem", "squid", "glow_squid", "cod", "salmon", "pufferfish",
    "tropical_fish", "dolphin", "tadpole", "bat", "happy_ghast", "nautilus",
];
const PROJECTILES: &[&str] = &[
    "arrow", "spectral_arrow", "trident", "snowball", "egg", "ender_pearl", "fireball", "small_fireball", "dragon_fireball", "wither_skull",
    "shulker_bullet", "llama_spit", "experience_bottle", "splash_potion", "lingering_potion", "firework_rocket", "wind_charge",
    "breeze_wind_charge", "fishing_bobber",
];
const MINECARTS: &[&str] = &["minecart", "chest_minecart", "furnace_minecart", "tnt_minecart", "hopper_minecart", "spawner_minecart", "command_block_minecart"];
const TAMEABLE: &[&str] = &["wolf", "cat", "parrot", "horse", "donkey", "mule", "llama", "trader_llama", "skeleton_horse", "zombie_horse", "camel", "camel_husk", "nautilus"];
const REMOVE_KINDS: &[&str] = &[
    "DROPS", "ARROWS", "BOATS", "MINECARTS", "XP", "PAINTINGS", "ITEMFRAMES", "ENDERCRYSTALS", "HOSTILE", "MONSTERS", "PASSIVE", "ANIMALS",
    "AMBIENT", "MOBS", "ENTITIES", "ALL", "CUSTOM", "TAMED", "NAMED",
];

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|k| format!("minecraft:{k}")).collect()
}

fn cmd_remove(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (world, radius) = match ctx.sender {
        Sender::Player(me) => {
            if args.is_empty() {
                return Err(CmdError::Usage(None));
            }
            let mut world = ctx.host.location(me).map(|l| l.world);
            let mut radius = 0i64;
            if args.len() >= 2 {
                match args[1].parse::<i32>() {
                    Ok(r) => radius = r as i64,
                    Err(_) => world = find_world(ctx.host, &args[1]),
                }
            }
            if args.len() >= 3 {
                radius = 0;
                world = find_world(ctx.host, &args[2]);
            }
            (world, radius)
        }
        Sender::Console => {
            if args.len() < 2 {
                return Err(CmdError::Usage(None));
            }
            (find_world(ctx.host, &args[1]), 0)
        }
    };
    let Some(world) = world else { return Err(CmdError::Tl("invalidWorld", vec![])) };
    let mut types = Vec::new();
    let mut custom = Vec::new();
    if args[0] == "*" || args[0] == "all" {
        types.push("ALL".to_string());
    } else {
        for t in args[0].split(',') {
            let up = t.to_uppercase();
            let plural = format!("{up}S");
            if REMOVE_KINDS.contains(&up.as_str()) {
                types.push(up);
            } else if REMOVE_KINDS.contains(&plural.as_str()) {
                types.push(plural);
            } else {
                types.push("CUSTOM".into());
                custom.push(t.to_string());
            }
        }
    }
    let mut kinds: Vec<String> = Vec::new();
    let mut warn = false;
    for c in &custom {
        match mob(c) {
            Some((_, k, _)) => kinds.push(format!("minecraft:{k}")),
            None => warn = true,
        }
    }
    if warn {
        ctx.tl("invalidMob", &[]);
    }
    let everything: Vec<String> = {
        let mut all = ctx.host.entity_types();
        if all.is_empty() {
            all = ids(MONSTERS).into_iter().chain(ids(ANIMALS)).chain(ids(PROJECTILES)).chain(ids(MINECARTS)).collect();
            all.extend(["minecraft:item", "minecraft:experience_orb", "minecraft:painting", "minecraft:item_frame", "minecraft:glow_item_frame", "minecraft:end_crystal", "minecraft:tnt", "minecraft:armor_stand"].map(String::from));
        }
        all.retain(|k| k != "minecraft:player");
        all
    };
    for t in &types {
        let add: Vec<String> = match t.as_str() {
            "DROPS" => ids(&["item"]),
            "ARROWS" => ids(PROJECTILES),
            "BOATS" => everything.iter().filter(|k| k.ends_with("_boat") || k.ends_with("_raft")).cloned().collect(),
            "MINECARTS" => ids(MINECARTS),
            "XP" => ids(&["experience_orb"]),
            "PAINTINGS" => ids(&["painting"]),
            "ITEMFRAMES" => ids(&["item_frame", "glow_item_frame"]),
            "ENDERCRYSTALS" => ids(&["end_crystal"]),
            "AMBIENT" => ids(&["ghast", "phantom", "happy_ghast"]),
            "HOSTILE" | "MONSTERS" => ids(MONSTERS),
            "PASSIVE" | "ANIMALS" => ids(ANIMALS),
            "MOBS" => ids(MONSTERS).into_iter().chain(ids(ANIMALS)).collect(),
            "ENTITIES" | "ALL" => everything.clone(),
            "TAMED" => ids(TAMEABLE),
            "NAMED" => ids(MONSTERS).into_iter().chain(ids(ANIMALS)).collect(),
            _ => Vec::new(),
        };
        for k in add {
            if !kinds.contains(&k) {
                kinds.push(k);
            }
        }
    }
    let removed = if kinds.is_empty() {
        0
    } else {
        // Flags for the glue: tamed and named mobs are skipped unless asked for.
        if !types.iter().any(|t| t == "TAMED") {
            kinds.push("#keep_tamed".into());
        } else if types.len() == 1 {
            kinds.push("#only_tamed".into());
        }
        if !types.iter().any(|t| t == "NAMED") {
            kinds.push("#keep_named".into());
        } else if types.len() == 1 {
            kinds.push("#only_named".into());
        }
        let center = match (ctx.sender, radius > 0) {
            (Sender::Player(me), true) => ctx.host.location(me),
            _ => None,
        };
        let r = (radius > 0).then_some(radius as f64);
        ctx.host.remove_entities(&world, center.as_ref(), r, &kinds)
    };
    ctx.tl("removed", &[Arg::Plain(removed.to_string())]);
    Ok(())
}

// ===== /editsign ====================================================================================

/// FormatUtil.formatString: & codes the user may use become § codes.
fn format_sign(ctx: &mut Ctx, text: &str) -> String {
    let color = ctx.has("essentials.editsign.color");
    let format = ctx.has("essentials.editsign.format");
    let magic = ctx.has("essentials.editsign.magic");
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' && i + 1 < chars.len() {
            let c = chars[i + 1].to_ascii_lowercase();
            let allowed = (color && c.is_ascii_hexdigit()) || (format && "lmnor".contains(c)) || (magic && c == 'k');
            if allowed {
                out.push('§');
                out.push(c);
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// FormatUtil.unformatString: § codes back to & codes.
fn unformat_sign(text: &str) -> String {
    text.replace('§', "&")
}

fn cmd_editsign(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() || (args.len() > 1 && args[1].parse::<i32>().is_err()) {
        return Err(CmdError::Usage(None));
    }
    let world = ctx.host.location(me).map(|l| l.world).unwrap_or_default();
    let pos = ctx.host.target_block(me, 5.0);
    let Some((pos, mut lines)) = pos.and_then(|p| ctx.host.sign_lines(&world, p).map(|l| (p, l))) else {
        return Err(CmdError::Tl("editsignCommandTarget", vec![]));
    };
    let line = args.get(1).and_then(|l| l.parse::<i32>().ok()).map(|l| l - 1);
    if let Some(l) = line {
        if !(0..4).contains(&l) && args[0].to_lowercase() != "frob" {
            if ["set", "clear", "copy", "paste"].contains(&args[0].to_lowercase().as_str()) && !(args[0].eq_ignore_ascii_case("set") && args.len() <= 2) {
                return Err(CmdError::Tl("editsignCommandNoLine", vec![]));
            }
        }
    }
    let label = ctx.label.clone();
    let sub = args[0].to_lowercase();
    if sub == "set" && args.len() > 2 {
        let l = line.unwrap_or(0) as usize;
        let text = format_sign(ctx, &final_arg(args, 2)).trim().to_string();
        if ess_text::strip_legacy(&text, false).chars().count() > 15 && !ctx.has("essentials.editsign.unlimited") {
            return Err(CmdError::Tl("editsignCommandLimit", vec![]));
        }
        lines[l] = text.clone();
        if commit_sign(ctx, me, &world, pos, lines) {
            ctx.tl("editsignCommandSetSuccess", &[Arg::Plain((l + 1).to_string()), Arg::Plain(text)]);
        }
    } else if sub == "clear" {
        match line {
            None => {
                if commit_sign(ctx, me, &world, pos, Default::default()) {
                    ctx.tl("editsignCommandClear", &[]);
                }
            }
            Some(l) => {
                lines[l as usize] = String::new();
                if commit_sign(ctx, me, &world, pos, lines) {
                    ctx.tl("editsignCommandClearLine", &[Arg::Plain((l + 1).to_string())]);
                }
            }
        }
    } else if sub == "copy" {
        let copy = ctx.ess.state::<WorldState>().sign_copy.entry(me).or_default();
        match line {
            None => {
                for i in 0..4 {
                    copy[i] = unformat_sign(&lines[i]);
                }
                ctx.tl("editsignCopy", &[Arg::Plain(label)]);
            }
            Some(l) => {
                copy[l as usize] = unformat_sign(&lines[l as usize]);
                ctx.tl("editsignCopyLine", &[Arg::Plain((l + 1).to_string()), Arg::Plain(label)]);
            }
        }
    } else if sub == "paste" {
        let copy = ctx.ess.state::<WorldState>().sign_copy.get(&me).cloned().unwrap_or_default();
        match line {
            None => {
                for i in 0..4 {
                    lines[i] = format_sign(ctx, &copy[i]);
                }
                if commit_sign(ctx, me, &world, pos, lines) {
                    ctx.tl("editsignPaste", &[Arg::Plain(label)]);
                }
            }
            Some(l) => {
                lines[l as usize] = format_sign(ctx, &copy[l as usize]);
                if commit_sign(ctx, me, &world, pos, lines) {
                    ctx.tl("editsignPasteLine", &[Arg::Plain((l + 1).to_string()), Arg::Plain(label)]);
                }
            }
        }
    } else {
        return Err(CmdError::Usage(None));
    }
    Ok(())
}

/// callSignEvent: listeners may veto or rewrite; true when the sign was written.
fn commit_sign(ctx: &mut Ctx, me: Uuid, world: &str, pos: [i32; 3], mut lines: [String; 4]) -> bool {
    if ctx.ess.on_sign_change(ctx.host, me, world, pos, &mut lines) == Allow::No {
        return false;
    }
    ctx.host.set_sign_lines(world, pos, &lines);
    true
}

// ===== Fun: /antioch /nuke /fireball /beezooka /kittycannon =========================================

fn cmd_antioch(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if !args.is_empty() {
        for line in ["...lobbest thou thy Holy Hand Grenade of Antioch towards thy foe,", "who being naughty in My sight, shall snuff it."] {
            let spans = ctx.ess.render(&ess_text::escape_tags(line));
            ctx.host.broadcast(&spans);
        }
    }
    let at = target(ctx, me, 300.0);
    ctx.host.spawn_entity("minecraft:tnt", &at, None);
    Ok(())
}

fn cmd_nuke(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let targets = if args.is_empty() {
        online_sorted(ctx.host)
    } else {
        let mut v = Vec::new();
        for i in 0..args.len() {
            v.push(get_player(ctx, args, i)?);
        }
        v
    };
    for p in targets {
        ctx.tl_to(Sender::Player(p), "nuke", &[]);
        let Some(at) = ctx.host.location(p) else { continue };
        let [bx, _, bz] = at.block();
        let top = ctx.host.highest_y(&at.world, bx, bz).unwrap_or(at.block()[1]);
        for x in (-10..=10).step_by(5) {
            for z in (-10..=10).step_by(5) {
                let loc = Loc::new(&at.world, (bx + x) as f64, (top + 64) as f64, (bz + z) as f64);
                ctx.host.spawn_entity("minecraft:tnt", &loc, Some("{Tags:[\"ess_tnt_proj\"]}"));
            }
        }
    }
    Ok(())
}

const FIREBALLS: &[(&str, &str)] = &[
    ("fireball", "fireball"), ("small", "small_fireball"), ("large", "fireball"), ("arrow", "arrow"), ("skull", "wither_skull"), ("egg", "egg"),
    ("snowball", "snowball"), ("expbottle", "experience_bottle"), ("dragon", "dragon_fireball"), ("splashpotion", "splash_potion"),
    ("lingeringpotion", "lingering_potion"), ("trident", "trident"), ("windcharge", "wind_charge"),
];

fn cmd_fireball(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let ty = args.first().filter(|a| FIREBALLS.iter().any(|(n, _)| n == &a.as_str())).map(String::as_str).unwrap_or("fireball").to_string();
    let mut speed = 2.0f64;
    if let Some(s) = args.get(1).and_then(|s| s.parse::<f64>().ok()).filter(|s| s.is_finite()) {
        speed = s.min(ctx.ess.config_f64("max-projectile-speed", 8.0)).max(0.0);
    }
    let node = format!("essentials.fireball.{ty}");
    if !ctx.has(&node) {
        return Err(CmdError::Tl("noPerm", vec![Arg::Plain(node)]));
    }
    let ride = args.get(2).is_some_and(|a| a.eq_ignore_ascii_case("ride")) && ctx.has("essentials.fireball.ride");
    let kind = FIREBALLS.iter().find(|(n, _)| *n == ty).map(|(_, k)| *k).unwrap_or("fireball");
    let (eye, dir) = eye_and_direction(ctx.host, me);
    let at = Loc { x: eye.x + dir[0] * speed, y: eye.y + dir[1] * speed, z: eye.z + dir[2] * speed, ..eye };
    let mut nbt = vec![motion(dir, speed), format!("Owner:{}", uuid_ints(me)), "Tags:[\"ess_fireball_proj\"]".to_string()];
    if kind.ends_with("fireball") || kind == "wither_skull" || kind == "wind_charge" {
        nbt.push(format!("power:[{:.4}d,{:.4}d,{:.4}d]", dir[0] * 0.1, dir[1] * 0.1, dir[2] * 0.1));
    }
    if kind == "arrow" || kind == "trident" {
        nbt.push("pickup:0b".into());
    }
    ctx.host.spawn_entity(&format!("minecraft:{kind}"), &at, Some(&format!("{{{}}}", nbt.join(","))));
    if ride {
        let name = ctx.name_of(me);
        ctx.host.console_command(&format!("ride {name} mount @e[tag=ess_fireball_proj,limit=1,sort=nearest]"));
    }
    Ok(())
}

fn launch(ctx: &mut Ctx, me: Uuid, kind: &str, extra: &str) {
    let (eye, dir) = eye_and_direction(ctx.host, me);
    let state = ctx.ess.state::<WorldState>();
    state.next_tag += 1;
    let tag = format!("ess_proj_{}", state.next_tag);
    let nbt = format!("{{{},Tags:[\"{tag}\"]{extra}}}", motion(dir, 2.0));
    ctx.host.spawn_entity(&format!("minecraft:{kind}"), &eye, Some(&nbt));
    let at = ctx.ess.scheduler.tick + 20;
    ctx.ess.state::<WorldState>().projectiles.push((at, eye.world.clone(), tag, kind.to_string()));
}

fn cmd_beezooka(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    launch(ctx, me, "bee", "");
    Ok(())
}

fn cmd_kittycannon(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    const CATS: &[&str] = &["tabby", "black", "red", "siamese", "british_shorthair", "calico", "persian", "ragdoll", "white", "jellie", "all_black"];
    let pick = (ctx.host.now_millis() as u64 ^ ctx.ess.scheduler.tick.wrapping_mul(0x9e37_79b9)) as usize % CATS.len();
    let extra = format!(",variant:\"minecraft:{}\",Owner:{},Age:-24000", CATS[pick], uuid_ints(me));
    launch(ctx, me, "cat", &extra);
    Ok(())
}

/// The exploding projectiles of /beezooka and /kittycannon, 20 ticks after launch.
fn fire_projectiles(ess: &mut Ess, host: &mut dyn Host) {
    let now = ess.scheduler.tick;
    let state = ess.state::<WorldState>();
    if state.projectiles.is_empty() {
        return;
    }
    let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut state.projectiles).into_iter().partition(|(at, ..)| *at <= now);
    state.projectiles = keep;
    for (_, world, tag, kind) in due {
        host.console_command(&format!(
            "execute in {world} as @e[tag={tag},limit=1] at @s run summon minecraft:creeper ~ ~ ~ {{Fuse:0s,ignited:1b,ExplosionRadius:0b,Silent:1b}}"
        ));
        host.remove_entities(&world, None, None, &[format!("minecraft:{kind}"), format!("#tag:{tag}")]);
    }
}

// ===== Death (EssentialsEntityListener.onPlayerDeath*) ==============================================

/// What happens to a dying player's things (keepinv, keepxp, curse policies).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeathPolicy {
    pub keep_inventory: bool,
    pub keep_level: bool,
    /// Items to drop although the inventory is kept (curses with policy `drop`).
    pub drop: Vec<Item>,
}

fn has_enchant(item: &Item, id: &str) -> bool {
    let e = item.components.get("enchantments").or_else(|| item.components.get("minecraft:enchantments"));
    let Some(e) = e else { return false };
    let map = e.get("levels").unwrap_or(e);
    map.get(id).is_some() || map.get(id.trim_start_matches("minecraft:")).is_some()
}

/// Called by the glue before vanilla drops a dying player's inventory: decides keepinv/keepxp and removes cursed
/// items per `vanishing-items-policy` / `binding-items-policy` (keep, drop, delete).
pub fn death_policy(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> DeathPolicy {
    let who = Sender::Player(player);
    let mut out = DeathPolicy { keep_inventory: ess.has(host, who, "essentials.keepinv"), keep_level: ess.has(host, who, "essentials.keepxp"), drop: Vec::new() };
    if !out.keep_inventory {
        return out;
    }
    let vanish = ess.config_str("vanishing-items-policy").unwrap_or_else(|| "keep".into()).to_lowercase();
    let bind = ess.config_str("binding-items-policy").unwrap_or_else(|| "keep".into()).to_lowercase();
    if vanish == "keep" && bind == "keep" {
        return out;
    }
    for (slot, item) in host.inventory(player).into_iter().enumerate() {
        let Some(item) = item else { continue };
        let policy = if vanish != "keep" && has_enchant(&item, "minecraft:vanishing_curse") {
            Some(&vanish)
        } else if bind != "keep" && has_enchant(&item, "minecraft:binding_curse") {
            Some(&bind)
        } else {
            None
        };
        if let Some(p) = policy {
            if p == "drop" {
                out.drop.push(item.clone());
            }
            host.set_slot(player, slot, None);
        }
    }
    out
}

// ===== Listeners ====================================================================================

/// LocationUtil.shouldFly: more than two unsafe (hollow) blocks below, or the void.
fn should_fly(host: &mut dyn Host, at: &Loc) -> bool {
    let [x, _, z] = at.block();
    let mut y = at.y.round() as i32;
    let (min, _) = host.height_range(&at.world);
    let mut count = 0;
    while y >= min && host.block(&at.world, [x, y - 1, z]).is_none_or(|b| is_hollow(&b) && !b.ends_with("water") && !b.ends_with("lava")) {
        y -= 1;
        count += 1;
        if count > 2 {
            return true;
        }
    }
    y < min
}

fn on_join(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _first: bool) {
    let who = Sender::Player(player);
    let flymode = ess.users.get(player).is_some_and(|u| u.bool("flymode"));
    let restore = flymode && ess.has(host, who, "essentials.fly");
    let announce = ess.config_bool("send-fly-enable-on-join", true);
    let enabled = Arg::Parsed(ess.tl("enabled", &[]));
    if restore {
        host.set_allow_flight(player, true);
        if announce {
            let name = Arg::Plain(display_name(ess, host, player));
            ess.send_tl(host, who, "flyMode", &[enabled.clone(), name]);
        }
    }
    if ess.has(host, who, "essentials.fly.safelogin") {
        if let Some(at) = host.location(player) {
            let in_water = host.block(&at.world, at.block()).is_some_and(|b| block_id(&b) == "minecraft:water");
            if !in_water && should_fly(host, &at) {
                host.set_allow_flight(player, true);
                host.set_flying(player, true);
                if !restore && announce {
                    let name = Arg::Plain(display_name(ess, host, player));
                    ess.send_tl(host, who, "flyMode", &[enabled, name]);
                }
            }
        }
    }
    if !ess.has(host, who, "essentials.speed") {
        host.set_fly_speed(player, 0.1);
        host.set_walk_speed(player, 0.2);
    }
    if flymode && !ess.has(host, who, "essentials.fly") {
        ess.users.user(player).set("flymode", json!(false));
        let name = host.name(player).unwrap_or_default();
        ess.send_mini(host, Sender::Console, &ess_text::escape_tags(&format!("Set fly mode to false for {name} because they had it enabled without permission.")));
    }
    if ess.users.get(player).is_some_and(|u| u.bool("godmode")) && !ess.has(host, who, "essentials.god") {
        ess.users.user(player).set("godmode", json!(false));
        let name = host.name(player).unwrap_or_default();
        ess.send_mini(host, Sender::Console, &ess_text::escape_tags(&format!("Set god mode to false for {name} because they had it enabled without permission.")));
    }
    if let Some(t) = ess.state::<WorldState>().ptime.get(&player).copied() {
        host.set_player_time(player, Some(t));
    }
}

fn on_quit(ess: &mut Ess, _host: &mut dyn Host, player: Uuid) {
    if ess.config_bool("remove-god-on-disconnect", false) && ess.users.get(player).is_some_and(|u| u.bool("godmode")) {
        ess.users.user(player).set("godmode", json!(false));
    }
    let state = ess.state::<WorldState>();
    state.flight.remove(&player);
    state.pweather.remove(&player);
    state.ptime.remove(&player);
}

fn on_moved(ess: &mut Ess, host: &mut dyn Host, player: Uuid, from: &Loc, to: &Loc) {
    if from.world == to.world {
        return;
    }
    let who = Sender::Player(player);
    if ess.config_bool("world-change-fly-reset", true) && !matches!(host.game_mode(player), GameMode::Creative | GameMode::Spectator) && !ess.has(host, who, "essentials.fly") {
        host.set_allow_flight(player, false);
    }
    if ess.config_bool("world-change-speed-reset", true) {
        if !ess.has(host, who, "essentials.speed") {
            host.set_fly_speed(player, 0.1);
            host.set_walk_speed(player, 0.2);
        } else if !ess.has(host, who, "essentials.speed.bypass") {
            let (walk, fly) = host.speeds(player);
            let max_walk = ess.config_f64("max-walk-speed", 0.8) as f32;
            let max_fly = ess.config_f64("max-fly-speed", 0.8) as f32;
            if fly > max_fly {
                host.set_fly_speed(player, max_fly);
            }
            if walk > max_walk {
                host.set_walk_speed(player, max_walk);
            }
        }
    }
    if ess.config_bool("world-change-preserve-flying", true) && ess.has(host, who, "essentials.fly") {
        if let Some((allow, flying)) = ess.state::<WorldState>().flight.get(&player).copied() {
            if allow {
                host.set_allow_flight(player, true);
                if flying {
                    host.set_flying(player, true);
                }
            }
        }
    }
    let world = world_name(&to.world);
    if ess.config_list("no-god-in-worlds").iter().any(|w| *w == world) && ess.users.get(player).is_some_and(|u| u.bool("godmode")) {
        ess.send_tl(host, who, "noGodWorldWarning", &[]);
    }
}

fn on_tick(ess: &mut Ess, host: &mut dyn Host) {
    for p in host.online() {
        if is_god(ess, host, p) && ess.users.get(p).is_some_and(|u| u.bool("godmode")) {
            let (food, sat) = host.food(p);
            if food < 20 || sat < 10.0 {
                host.set_food(p, 20, 10.0);
            }
        }
        let flight = (host.allow_flight(p), host.flying(p));
        ess.state::<WorldState>().flight.insert(p, flight);
    }
    fire_projectiles(ess, host);
    // One poll for every module's HTTP answers (ids carry the module's tag in their high bits).
    for (id, status, body) in host.http_poll() {
        let _ = crate::game::ess_geoip::on_http(ess, host, id, status, &body) || crate::game::ess_discord::on_http(ess, host, id, status, &body);
    }
}

fn on_damage(ess: &mut Ess, host: &mut dyn Host, player: Uuid, cause: &str, _amount: f64) -> Allow {
    if is_god(ess, host, player) {
        host.set_fire_ticks(player, 0);
        return Allow::No;
    }
    if let Some(attacker) = cause.strip_prefix("player:").and_then(parse_uuid) {
        if attacker != player && is_god(ess, host, attacker) && !ess.has(host, Sender::Player(attacker), "essentials.god.pvp") {
            return Allow::No;
        }
    }
    Allow::Yes
}
#[cfg(test)]
mod tests {
    use crate::game::ess_core::*;
    use crate::game::ess_testkit::*;
    use serde_json::json;

    fn server() -> (TestServer, Uuid, Uuid) {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bobby");
        t.op("Alice");
        (t, alice, bob)
    }

    /// A server where `Carol` is a regular player with exactly these extra nodes.
    fn with_player(nodes: &[&str], config: &str) -> (TestServer, Uuid) {
        let mut t = TestServer::with_config(config);
        let list = nodes.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(", ");
        t.permissions(&format!("groups:\n  default:\n    default: true\n    permissions: [{list}]\n"));
        let carol = t.join("Carol");
        (t, carol)
    }

    // ----- fly ---------------------------------------------------------------------------------

    #[test]
    fn fly_toggles_and_persists() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/fly"), ["Set fly mode enabled for Alice."]);
        assert!(t.host.p(alice).allow_flight);
        assert_eq!(t.userdata(alice)["flymode"], json!(true));
        t.host.p(alice).flying = true;
        assert_eq!(t.cmd(alice, "/fly"), ["Set fly mode disabled for Alice."]);
        assert!(!t.host.p(alice).allow_flight);
        assert!(!t.host.p(alice).flying);
        assert_eq!(t.cmd(alice, "/fly on"), ["Set fly mode enabled for Alice."]);
        assert_eq!(t.cmd(alice, "/fly enable"), ["Set fly mode enabled for Alice."]);
        assert_eq!(t.cmd(alice, "/fly 0"), ["Set fly mode disabled for Alice."]);
        // Others: both get the line.
        assert_eq!(t.cmd(alice, "/fly Bobby"), ["Set fly mode enabled for Bobby."]);
        assert_eq!(t.said(bob), ["Set fly mode enabled for Bobby."]);
        assert_eq!(t.cmd(alice, "/fly Bob off"), ["Set fly mode disabled for Bobby."]);
        assert!(!t.host.p(bob).allow_flight);
        assert_eq!(t.cmd(alice, "/fly zz"), ["Error: Player not found."]);
        assert_eq!(t.cmd(alice, "/fly x"), ["Error: Player not found."]);
        assert_eq!(t.console("fly Bobby"), ["Set fly mode enabled for Bobby."]);
        assert_eq!(t.cmd(bob, "/fly"), ["You do not have access to that command."]);
    }

    #[test]
    fn fly_without_others_permission_toggles_self() {
        let (mut t, carol) = with_player(&["essentials.fly"], "");
        let _dave = t.join("Dave");
        assert_eq!(t.cmd(carol, "/fly Dave"), ["Set fly mode enabled for Carol."]);
        assert_eq!(t.cmd(carol, "/fly Dave on"), ["Set fly mode disabled for Carol."]);
    }

    #[test]
    fn fly_restored_on_join_and_safelogin() {
        let (mut t, alice, _bob) = server();
        t.cmd(alice, "/fly");
        t.quit(alice);
        t.host.p(alice).allow_flight = false;
        t.join("Alice");
        assert!(t.host.p(alice).allow_flight);
        assert_eq!(t.said(alice), ["Set fly mode enabled for Alice."]);
        // safelogin: high in the air -> flying.
        t.cmd(alice, "/fly off");
        t.quit(alice);
        t.host.p(alice).loc = Loc::new("minecraft:overworld", 5.5, 100.0, 5.5);
        t.join("Alice");
        assert!(t.host.p(alice).allow_flight && t.host.p(alice).flying);
        assert_eq!(t.said(alice), ["Set fly mode enabled for Alice."]);
        // On the ground: nothing.
        t.quit(alice);
        t.host.p(alice).allow_flight = false;
        t.host.p(alice).flying = false;
        t.host.p(alice).loc = Loc::new("minecraft:overworld", 5.5, 64.0, 5.5);
        t.join("Alice");
        assert!(!t.host.p(alice).flying);
        assert!(t.said(alice).is_empty());
    }

    #[test]
    fn fly_join_message_can_be_off_and_fly_dropped_without_permission() {
        let (mut t, carol) = with_player(&["essentials.fly"], "send-fly-enable-on-join: false");
        t.cmd(carol, "/fly");
        t.quit(carol);
        t.join("Carol");
        assert!(t.host.p(carol).allow_flight);
        assert!(t.said(carol).is_empty());
        t.quit(carol);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: []\n");
        t.host.p(carol).allow_flight = false;
        t.join("Carol");
        assert!(!t.host.p(carol).allow_flight);
        assert_eq!(t.userdata(carol)["flymode"], json!(false));
    }

    #[test]
    fn world_change_resets_flight_and_speed() {
        let (mut t, carol) = with_player(&[], "");
        t.host.p(carol).allow_flight = true;
        t.host.p(carol).fly_speed = 0.5;
        t.host.p(carol).walk_speed = 0.6;
        t.walk(carol, Loc::new("minecraft:the_nether", 0.5, 70.0, 0.5));
        assert!(!t.host.p(carol).allow_flight);
        assert_eq!(t.host.p(carol).fly_speed, 0.1);
        assert_eq!(t.host.p(carol).walk_speed, 0.2);
        // Creative keeps flight; world-change-fly-reset false keeps it too.
        let (mut t, carol) = with_player(&[], "world-change-fly-reset: false\nworld-change-speed-reset: false");
        t.host.p(carol).allow_flight = true;
        t.host.p(carol).walk_speed = 0.6;
        t.walk(carol, Loc::new("minecraft:the_nether", 0.5, 70.0, 0.5));
        assert!(t.host.p(carol).allow_flight);
        assert_eq!(t.host.p(carol).walk_speed, 0.6);
        // With essentials.speed the speed is capped at max-walk-speed.
        let (mut t, carol) = with_player(&["essentials.speed"], "");
        t.host.p(carol).walk_speed = 0.95;
        t.walk(carol, Loc::new("minecraft:the_end", 0.5, 70.0, 0.5));
        assert!((t.host.p(carol).walk_speed - 0.8).abs() < 1e-6);
    }

    #[test]
    fn speed_reset_on_join_without_permission() {
        let (mut t, carol) = with_player(&[], "");
        t.quit(carol);
        t.host.p(carol).walk_speed = 0.7;
        t.host.p(carol).fly_speed = 0.7;
        t.join("Carol");
        assert_eq!(t.host.p(carol).walk_speed, 0.2);
        assert_eq!(t.host.p(carol).fly_speed, 0.1);
    }

    // ----- god -----------------------------------------------------------------------------------

    #[test]
    fn god_toggles_heals_and_blocks_damage() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).health = 4.0;
        t.host.p(alice).food = 3;
        assert_eq!(t.cmd(alice, "/god"), ["God mode enabled."]);
        assert_eq!(t.host.p(alice).health, 20.0);
        assert_eq!(t.host.p(alice).food, 20);
        assert_eq!(t.userdata(alice)["godmode"], json!(true));
        t.host.p(alice).fire = 100;
        assert_eq!(t.ess.on_damage(&mut t.host, alice, "fall", 5.0), Allow::No);
        assert_eq!(t.host.p(alice).fire, 0);
        assert_eq!(t.ess.on_damage(&mut t.host, bob, "fall", 5.0), Allow::Yes);
        // No hunger.
        t.host.p(alice).food = 12;
        t.tick(1);
        assert_eq!(t.host.p(alice).food, 20);
        assert_eq!(t.host.p(alice).saturation, 10.0);
        assert_eq!(t.cmd(alice, "/god"), ["God mode disabled."]);
        assert_eq!(t.ess.on_damage(&mut t.host, alice, "fall", 5.0), Allow::Yes);
        assert_eq!(t.cmd(alice, "/tgm Bobby"), ["God mode enabled."]);
        assert_eq!(t.said(bob), ["God mode enabled."]);
        assert_eq!(t.cmd(bob, "/god"), ["You do not have access to that command."]);
        assert_eq!(t.console("godmode Bobby off"), ["God mode disabled."]);
    }

    #[test]
    fn god_attacker_needs_god_pvp() {
        let (mut t, carol) = with_player(&["essentials.god"], "");
        let dave = t.join("Dave");
        t.cmd(carol, "/god");
        let cause = format!("player:{}", uuid_string(carol));
        assert_eq!(t.ess.on_damage(&mut t.host, dave, &cause, 2.0), Allow::No);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.god, essentials.god.pvp]\n");
        assert_eq!(t.ess.on_damage(&mut t.host, dave, &cause, 2.0), Allow::Yes);
    }

    #[test]
    fn no_god_worlds_and_remove_on_disconnect() {
        let (mut t, carol) = with_player(&["essentials.god"], "no-god-in-worlds: [world_nether]\nremove-god-on-disconnect: true");
        t.cmd(carol, "/god");
        t.walk(carol, Loc::new("minecraft:the_nether", 0.5, 70.0, 0.5));
        assert_eq!(t.said(carol), ["Warning! God mode in this world disabled."]);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "fall", 5.0), Allow::Yes);
        t.walk(carol, Loc::new("minecraft:overworld", 0.5, 70.0, 0.5));
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "fall", 5.0), Allow::No);
        t.quit(carol);
        assert_eq!(t.userdata(carol)["godmode"], json!(false));
    }

    #[test]
    fn god_dropped_on_join_without_permission() {
        let (mut t, carol) = with_player(&["essentials.god"], "");
        t.cmd(carol, "/god");
        t.quit(carol);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: []\n");
        t.join("Carol");
        assert_eq!(t.userdata(carol)["godmode"], json!(false));
    }

    // ----- heal, feed, ext, burn, ice, rest, suicide -----------------------------------------

    #[test]
    fn heal_self_others_and_dead() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).health = 5.0;
        t.host.p(alice).food = 2;
        t.host.p(alice).fire = 60;
        assert_eq!(t.cmd(alice, "/heal"), ["You have been healed."]);
        let p = t.host.p(alice).clone();
        assert_eq!((p.health, p.food, p.fire), (20.0, 20, 0));
        assert!(t.host.console_commands.iter().any(|c| c == "effect clear Alice"));
        t.host.p(bob).health = 1.0;
        assert_eq!(t.cmd(alice, "/heal Bobby"), ["Healed Bobby."]);
        assert_eq!(t.said(bob), ["You have been healed."]);
        assert_eq!(t.host.p(bob).health, 20.0);
        t.host.p(bob).health = 0.0;
        assert_eq!(t.cmd(alice, "/heal Bobby"), ["Error: You cannot heal someone who is dead!"]);
        assert_eq!(t.console("heal Bobby"), ["Error: You cannot heal someone who is dead!"]);
        let usage = t.console("heal");
        assert_eq!(usage[0], "Command Help: /heal");
        assert_eq!(t.cmd(alice, "/heal nobodyhere"), ["Error: Player not found."]);
    }

    #[test]
    fn heal_cooldown_and_effects_option() {
        let (mut t, carol) = with_player(&["essentials.heal"], "remove-effects-on-heal: false");
        assert_eq!(t.cmd(carol, "/heal"), ["You have been healed."]);
        assert!(t.host.console_commands.is_empty());
        assert_eq!(t.cmd(carol, "/heal"), ["Error: Time before next heal: 1 minute."]);
        t.host.now += 30_000;
        assert_eq!(t.cmd(carol, "/heal"), ["Error: Time before next heal: 30 seconds."]);
        t.host.now += 31_000;
        assert_eq!(t.cmd(carol, "/heal"), ["You have been healed."]);
        // Without heal.others, an argument heals yourself.
        t.host.now += 61_000;
        let _dave = t.join("Dave");
        assert_eq!(t.cmd(carol, "/heal Dave"), ["You have been healed."]);
    }

    #[test]
    fn feed_self_and_others() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).food = 3;
        t.host.p(alice).saturation = 0.0;
        assert_eq!(t.cmd(alice, "/feed"), ["Your appetite was sated."]);
        assert_eq!((t.host.p(alice).food, t.host.p(alice).saturation), (20, 10.0));
        t.host.p(bob).food = 1;
        assert_eq!(t.cmd(alice, "/eat Bobby"), ["You satiated the appetite of Bobby."]);
        assert!(t.said(bob).is_empty());
        assert_eq!(t.host.p(bob).food, 20);
        assert_eq!(t.console("feed *"), ["You satiated the appetite of Alice.", "You satiated the appetite of Bobby."]);
        assert_eq!(t.console("feed")[0], "Command Help: /feed");
    }

    #[test]
    fn feed_shares_the_heal_cooldown() {
        let (mut t, carol) = with_player(&["essentials.feed", "essentials.heal"], "");
        assert_eq!(t.cmd(carol, "/feed"), ["Your appetite was sated."]);
        assert_eq!(t.cmd(carol, "/heal"), ["Error: Time before next heal: 1 minute."]);
    }

    #[test]
    fn ext_and_burn() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).fire = 80;
        assert_eq!(t.cmd(alice, "/ext"), ["You extinguished yourself."]);
        assert_eq!(t.host.p(alice).fire, 0);
        t.host.p(bob).fire = 80;
        assert_eq!(t.cmd(alice, "/extinguish Bobby"), ["You extinguished Bobby."]);
        assert_eq!(t.host.p(bob).fire, 0);
        assert_eq!(t.cmd(alice, "/burn Bobby 5"), ["You set Bobby on fire for 5 seconds."]);
        assert_eq!(t.host.p(bob).fire, 100);
        assert_eq!(t.cmd(alice, "/burn Bobby -3"), ["You set Bobby on fire for 0 seconds."]);
        assert_eq!(t.host.p(bob).fire, 0);
        assert_eq!(t.cmd(alice, "/burn Bobby")[0], "Command Help: /burn");
        assert_eq!(t.cmd(alice, "/burn Nobody 3"), ["Error: Player not found."]);
        assert_eq!(t.console("ext")[0], "Command Help: /ext");
    }

    #[test]
    fn ice_and_rest() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/ice"), ["You feel much colder..."]);
        assert_eq!(t.host.p(alice).freeze, 140);
        assert_eq!(t.cmd(alice, "/efreeze Bobby"), ["Chilling Bobby."]);
        assert_eq!(t.said(bob), ["You feel much colder..."]);
        assert_eq!(t.host.p(bob).freeze, 140);
        assert_eq!(t.console("ice")[0], "Command Help: /ice");
        t.host.p(alice).stats.insert("minecraft:custom/minecraft:time_since_rest".into(), 90000);
        assert_eq!(t.cmd(alice, "/rest"), ["You feel well rested."]);
        assert_eq!(t.host.p(alice).stats["minecraft:custom/minecraft:time_since_rest"], 0);
        assert_eq!(t.cmd(alice, "/rest Bobby"), ["Resting Bobby."]);
        assert_eq!(t.said(bob), ["You feel well rested."]);
    }

    #[test]
    fn suicide_kills_and_broadcasts() {
        let (mut t, _alice, bob) = server();
        let carol = t.join("Carol");
        assert_eq!(t.cmd(carol, "/suicide"), ["Goodbye cruel world...", "Player Carol took their own life."]);
        assert_eq!(t.host.p(carol).health, 0.0);
        assert_eq!(t.said(bob), ["Player Carol took their own life."]);
    }

    // ----- gamemode ----------------------------------------------------------------------------

    #[test]
    fn gamemode_labels_args_and_others() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/gmc"), ["Set game mode creative for Alice."]);
        assert_eq!(t.host.p(alice).mode, GameMode::Creative);
        assert_eq!(t.cmd(alice, "/gamemode survival"), ["Set game mode survival for Alice."]);
        assert_eq!(t.cmd(alice, "/gm 2"), ["Set game mode adventure for Alice."]);
        assert_eq!(t.cmd(alice, "/spectator"), ["Set game mode spectator for Alice."]);
        assert_eq!(t.cmd(alice, "/gms"), ["Set game mode survival for Alice."]);
        assert_eq!(t.cmd(alice, "/gmt"), ["Set game mode creative for Alice."]);
        assert_eq!(t.cmd(alice, "/gmt"), ["Set game mode adventure for Alice."]);
        assert_eq!(t.cmd(alice, "/gmt"), ["Set game mode survival for Alice."]);
        assert_eq!(t.cmd(alice, "/gamemode creative Bobby"), ["Set game mode creative for Bobby."]);
        assert_eq!(t.host.p(bob).mode, GameMode::Creative);
        assert!(t.said(bob).is_empty());
        assert_eq!(t.cmd(alice, "/gma Bobby"), ["Set game mode adventure for Bobby."]);
        assert_eq!(t.cmd(alice, "/gamemode creative Nobody"), ["Error: Player not found."]);
        assert_eq!(t.cmd(alice, "/gamemode")[0], "Command Help: /gamemode");
        assert_eq!(t.console("gamemode spectator Bobby"), ["Set game mode spectator for Bobby."]);
        assert_eq!(t.console("gms Bobby"), ["Set game mode survival for Bobby."]);
        assert_eq!(t.console("gamemode banana Bobby")[0], "Command Help: /gamemode");
        let lines = t.console("gamemode toggle Bobby");
        assert_eq!(lines[0], "Command Help: /gamemode");
        assert_eq!(lines.last().unwrap(), "You need to specify a valid player/mode.");
    }

    #[test]
    fn gamemode_per_mode_permissions() {
        let (mut t, carol) = with_player(&["essentials.gamemode", "essentials.gamemode.survival"], "");
        assert_eq!(t.cmd(carol, "/gmc"), ["You do not have permission to change to gamemode creative"]);
        assert_eq!(t.host.p(carol).mode, GameMode::Survival);
        assert_eq!(t.cmd(carol, "/gms"), ["Set game mode survival for Carol."]);
        assert_eq!(t.cmd(carol, "/gamemode Dave")[0], "Command Help: /gamemode");
    }

    // ----- speed -------------------------------------------------------------------------------

    #[test]
    fn speed_walk_fly_caps_and_others() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/speed 2"), ["Set walking speed to 2 for Alice."]);
        // Ops bypass the cap: (2-1)/9 * (1-0.2) + 0.2.
        assert!((t.host.p(alice).walk_speed - (0.2 + 0.8 / 9.0)).abs() < 1e-5);
        t.host.p(alice).flying = true;
        t.host.p(alice).allow_flight = true;
        assert_eq!(t.cmd(alice, "/speed 0.5"), ["Set flying speed to 0.5 for Alice."]);
        assert!((t.host.p(alice).fly_speed - 0.05).abs() < 1e-6);
        assert_eq!(t.cmd(alice, "/walkspeed 1"), ["Set walking speed to 1 for Alice."]);
        assert!((t.host.p(alice).walk_speed - 0.2).abs() < 1e-6);
        assert_eq!(t.cmd(alice, "/fspeed 20"), ["Set flying speed to 10 for Alice."]);
        assert!((t.host.p(alice).fly_speed - 1.0).abs() < 1e-6);
        assert_eq!(t.cmd(alice, "/speed fly 1.5 Bobby"), ["Set flying speed to 1.5 for Bobby."]);
        assert!(t.said(bob).is_empty());
        assert_eq!(t.cmd(alice, "/speed walk 3 B"), ["Error: Player not found."]);
        assert_eq!(t.cmd(alice, "/speed jump 3")[0], "Command Help: /speed");
        assert_eq!(t.cmd(alice, "/speed abc")[0], "Command Help: /speed");
        assert_eq!(t.console("speed walk 2 Bobby"), ["Set walking speed to 2 for Bobby."]);
    }

    #[test]
    fn speed_cap_without_bypass_and_mode_permissions() {
        let (mut t, carol) = with_player(&["essentials.speed", "essentials.speed.walk"], "max-walk-speed: 0.5");
        assert_eq!(t.cmd(carol, "/speed 10"), ["Set walking speed to 10 for Carol."]);
        assert!((t.host.p(carol).walk_speed - 0.5).abs() < 1e-6);
        // Only walk allowed: a fly request becomes walk.
        assert_eq!(t.cmd(carol, "/speed fly 1"), ["Set walking speed to 1 for Carol."]);
    }

    // ----- time, ptime ---------------------------------------------------------------------------

    #[test]
    fn time_show_set_add_and_labels() {
        let (mut t, alice, _bob) = server();
        assert_eq!(t.cmd(alice, "/time"), ["06:00 or 6:00 AM or 0 ticks"]);
        assert_eq!(t.cmd(alice, "/time set day"), ["The time was set to 0 ticks in: world."]);
        assert_eq!(t.host.time["minecraft:overworld"], 24000);
        assert_eq!(t.cmd(alice, "/night"), ["The time was set to 14000 ticks in: world."]);
        assert_eq!(t.host.time["minecraft:overworld"], 62000);
        assert_eq!(t.cmd(alice, "/time add 1000"), ["The time was moved forward by 1000 ticks in: world."]);
        assert_eq!(t.host.time["minecraft:overworld"], 63000);
        assert_eq!(t.cmd(alice, "/time"), ["21:00 or 9:00 PM or 15000 ticks"]);
        assert_eq!(t.cmd(alice, "/time 6pm"), ["The time was set to 12000 ticks in: world."]);
        assert_eq!(t.cmd(alice, "/time 13:30"), ["The time was set to 7500 ticks in: world."]);
        assert_eq!(t.cmd(alice, "/time set noon world_nether"), ["The time was set to 6000 ticks in: world_nether."]);
        assert_eq!(t.cmd(alice, "/time set noon nowhere"), ["Error: Invalid world."]);
        assert_eq!(t.cmd(alice, "/time set banana")[0], "Command Help: /time");
        assert_eq!(t.console("time set 1000 all"), ["The time was set to 1000 ticks in: world, world_nether, world_the_end."]);
        let lines = t.console("time");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "The current time in world is 07:00 or 7:00 AM or 1000 ticks.");
    }

    #[test]
    fn time_permissions() {
        let (mut t, carol) = with_player(&["essentials.time"], "");
        assert_eq!(t.cmd(carol, "/time"), ["06:00 or 6:00 AM or 0 ticks"]);
        assert_eq!(t.cmd(carol, "/time set day"), ["Error: You are not authorized to set the time."]);
        let (mut t, carol) = with_player(&["essentials.time", "essentials.time.set"], "world-time-permissions: true");
        assert_eq!(t.cmd(carol, "/day"), ["Error: You are not authorized to set the time in world 'world'."]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.time, essentials.time.set, essentials.time.world.world]\n");
        assert_eq!(t.cmd(carol, "/day"), ["The time was set to 0 ticks in: world."]);
    }

    #[test]
    fn ptime_set_fixed_get_reset() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/ptime"), ["Alice's time is normal and matches the server."]);
        assert_eq!(t.cmd(alice, "/ptime noon"), ["Player time is set to 12:00 or 12:00 PM or 6000 ticks for: Alice."]);
        assert_eq!(t.host.p(alice).ptime, Some((30000, true)));
        assert_eq!(t.cmd(alice, "/ptime get"), ["Alice's time is 12:00 or 12:00 PM or 6000 ticks."]);
        assert_eq!(t.cmd(alice, "/ptime @night Bobby"), ["Player time is fixed to 20:00 or 8:00 PM or 14000 ticks for: Bobby."]);
        assert_eq!(t.host.p(bob).ptime, Some((38000, false)));
        assert_eq!(t.cmd(alice, "/ptime list Bobby"), ["Bobby's time is fixed to 20:00 or 8:00 PM or 14000 ticks."]);
        assert_eq!(t.cmd(alice, "/ptime reset *"), ["Player time has been reset for: Alice, Bobby"]);
        assert_eq!(t.host.p(bob).ptime, None);
        assert_eq!(t.cmd(alice, "/ptime banana")[0], "Command Help: /ptime");
        assert_eq!(t.console("ptime get")[0], "Command Help: /ptime");
        assert_eq!(t.console("ptime 1000 Bobby"), ["Player time is set to 07:00 or 7:00 AM or 1000 ticks for: Bobby."]);
    }

    #[test]
    fn ptime_others_needs_permission() {
        let (mut t, carol) = with_player(&["essentials.ptime"], "");
        t.join("Dave");
        assert_eq!(t.cmd(carol, "/ptime noon Dave"), ["You are not authorized to set other players' time."]);
        assert_eq!(t.cmd(carol, "/ptime noon Carol"), ["Player time is set to 12:00 or 12:00 PM or 6000 ticks for: Carol."]);
    }

    // ----- weather, pweather, thunder -----------------------------------------------------------

    #[test]
    fn weather_player_and_console() {
        let (mut t, alice, _bob) = server();
        assert_eq!(t.cmd(alice, "/weather storm"), ["You set the weather to storm in world."]);
        assert_eq!(t.host.weather["minecraft:overworld"], (true, false, None));
        assert_eq!(t.cmd(alice, "/sun"), ["You set the weather to sun in world."]);
        assert_eq!(t.host.weather["minecraft:overworld"].0, false);
        assert_eq!(t.cmd(alice, "/rain 60"), ["You set the weather to sun in world."]);
        assert_eq!(t.cmd(alice, "/weather storm 60"), ["You set the weather to storm in world for 60 seconds."]);
        assert_eq!(t.host.weather["minecraft:overworld"], (true, false, Some(1200)));
        assert_eq!(t.cmd(alice, "/weather sun 10"), ["You set the weather to sun in world for 10 seconds."]);
        assert_eq!(t.cmd(alice, "/weather")[0], "Command Help: /weather");
        assert_eq!(t.console("weather world_nether storm"), ["You set the weather to storm in world_nether."]);
        assert_eq!(t.console("weather sun"), ["Error: When running from console, usage is: /weather <world> <storm/sun> [duration]"]);
        assert_eq!(t.console("weather nowhere sun"), ["Error: World named nowhere not found!"]);
    }

    #[test]
    fn thunder_on_off() {
        let (mut t, alice, _bob) = server();
        assert_eq!(t.cmd(alice, "/thunder true"), ["You enabled thunder in your world."]);
        assert!(t.host.weather["minecraft:overworld"].1);
        assert_eq!(t.cmd(alice, "/thunder false 10"), ["You disabled thunder in your world for 10 seconds."]);
        assert_eq!(t.host.weather["minecraft:overworld"], (false, false, Some(200)));
        assert_eq!(t.cmd(alice, "/thunder")[0], "Command Help: /thunder");
    }

    #[test]
    fn pweather_set_get_reset() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/pweather"), ["Alice's weather is normal and matches the server."]);
        assert_eq!(t.cmd(alice, "/pweather storm"), ["Player weather is set to storm for: Alice."]);
        assert_eq!(t.host.p(alice).pweather, Some(true));
        assert_eq!(t.cmd(alice, "/pweather show"), ["Alice's weather is downfall."]);
        assert_eq!(t.cmd(alice, "/pweather sun Bobby"), ["Player weather is set to sun for: Bobby."]);
        assert_eq!(t.host.p(bob).pweather, Some(false));
        assert_eq!(t.cmd(alice, "/pweather get Bobby"), ["Bobby's weather is clear."]);
        assert_eq!(t.cmd(alice, "/pweather reset *"), ["Player weather has been reset for: Alice, Bobby"]);
        let lines = t.cmd(alice, "/pweather banana");
        assert_eq!(lines[0], "Command Help: /pweather");
        assert_eq!(lines.last().unwrap(), "Invalid weather type");
        assert_eq!(t.console("pweather get *"), ["These players have their own weather:", "Alice's weather is normal and matches the server.", "Bobby's weather is normal and matches the server."]);
    }

    #[test]
    fn pweather_others_needs_permission() {
        let (mut t, carol) = with_player(&["essentials.pweather"], "");
        t.join("Dave");
        assert_eq!(t.cmd(carol, "/pweather storm Dave"), ["You are not authorized to set other players' weather."]);
    }

    // ----- lightning ---------------------------------------------------------------------------

    #[test]
    fn lightning_target_and_players() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).target = Some([10, 63, 10]);
        assert!(t.cmd(alice, "/lightning").is_empty());
        assert_eq!(t.host.lightning.len(), 1);
        assert_eq!(t.host.lightning[0].0.block(), [10, 63, 10]);
        assert!(!t.host.lightning[0].1);
        assert_eq!(t.cmd(alice, "/smite Bobby"), ["Smiting Bobby"]);
        assert_eq!(t.said(bob), ["Thou hast been smitten!"]);
        assert!(t.host.lightning[1].1);
        assert!(t.host.console_commands.iter().any(|c| c == "damage Bobby 5 minecraft:lightning_bolt"));
        t.host.console_commands.clear();
        assert_eq!(t.cmd(alice, "/lightning Bobby 9"), ["Smiting Bobby"]);
        assert!(t.host.console_commands.iter().any(|c| c == "damage Bobby 9 minecraft:lightning_bolt"));
        t.host.console_commands.clear();
        t.cmd(bob, "/ping");
        t.ess.users.user(bob).set("godmode", json!(true));
        t.cmd(alice, "/lightning Bobby");
        assert!(!t.host.console_commands.iter().any(|c| c.starts_with("damage")));
        assert_eq!(t.console("lightning")[0], "Command Help: /lightning");
    }

    #[test]
    fn lightning_warn_off() {
        let mut t = TestServer::with_config("warn-on-smite: false");
        let alice = t.join("Alice");
        let bob = t.join("Bobby");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/lightning Bobby"), ["Smiting Bobby"]);
        assert!(t.said(bob).is_empty());
    }

    // ----- tree, bigtree, break ----------------------------------------------------------------

    #[test]
    fn tree_kinds_and_failures() {
        let (mut t, alice, _bob) = server();
        t.host.p(alice).target = Some([4, 63, 4]);
        assert_eq!(t.cmd(alice, "/tree tree"), ["Tree spawned."]);
        assert_eq!(t.host.console_commands.last().unwrap(), "execute in minecraft:overworld run place feature minecraft:oak 4 64 4");
        assert_eq!(t.cmd(alice, "/tree redwood"), ["Tree spawned."]);
        assert!(t.host.console_commands.last().unwrap().contains("minecraft:spruce"));
        assert_eq!(t.cmd(alice, "/tree jungle"), ["Tree spawned."]);
        assert!(t.host.console_commands.last().unwrap().contains("minecraft:jungle_tree_no_vine"));
        assert_eq!(t.cmd(alice, "/tree tallbirch"), ["Tree spawned."]);
        assert!(t.host.console_commands.last().unwrap().contains("minecraft:super_birch_bees_0002"));
        assert_eq!(t.cmd(alice, "/tree palm")[0], "Command Help: /tree");
        assert_eq!(t.cmd(alice, "/tree")[0], "Command Help: /tree");
        t.host.blocks.insert(("minecraft:overworld".into(), [4, 64, 4]), "minecraft:stone".into());
        assert_eq!(t.cmd(alice, "/tree birch"), ["Error: Tree generation failure. Try again on grass or dirt."]);
        assert_eq!(t.cmd(alice, "/bigtree tree"), ["Error: Big tree generation failure. Try again on grass or dirt."]);
        t.host.blocks.remove(&("minecraft:overworld".to_string(), [4, 64, 4]));
        assert_eq!(t.cmd(alice, "/bigtree redwood"), ["Big tree spawned."]);
        assert!(t.host.console_commands.last().unwrap().contains("minecraft:pine"));
        assert_eq!(t.cmd(alice, "/largetree darkoak"), ["Big tree spawned."]);
        assert!(t.host.console_commands.last().unwrap().contains("minecraft:dark_oak"));
        assert_eq!(t.cmd(alice, "/bigtree birch")[0], "Command Help: /bigtree");
        assert_eq!(t.console("tree oak"), ["Error: Only in-game players can use /tree."]);
    }

    #[test]
    fn break_target_block() {
        let (mut t, alice, _bob) = server();
        t.host.p(alice).target = Some([1, 63, 1]);
        assert!(t.cmd(alice, "/break").is_empty());
        assert_eq!(t.host.blocks[&("minecraft:overworld".to_string(), [1, 63, 1])], "minecraft:air");
        assert!(t.cmd(alice, "/break").is_empty());
        t.host.p(alice).target = Some([1, -64, 1]);
        assert!(t.cmd(alice, "/break").is_empty());
        let (mut t, carol) = with_player(&["essentials.break"], "");
        t.host.p(carol).target = Some([1, -64, 1]);
        assert_eq!(t.cmd(carol, "/break"), ["Error: You are not allowed to destroy bedrock."]);
        t.host.p(carol).target = Some([1, 63, 1]);
        // AntiBuild: Carol has no essentials.build: the break event is cancelled.
        t.cmd(carol, "/break");
        assert_eq!(t.host.blocks.get(&("minecraft:overworld".to_string(), [1, 63, 1])), None);
    }

    // ----- spawnmob, spawner, remove -------------------------------------------------------------

    #[test]
    fn spawnmob_counts_limits_stacks_and_data() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).target = Some([3, 63, 3]);
        assert_eq!(t.cmd(alice, "/spawnmob zombie"), ["1 zombies spawned"]);
        assert_eq!(t.host.entities.len(), 1);
        assert_eq!(t.host.entities[0].0, "minecraft:zombie");
        assert_eq!(t.host.entities[0].1.block(), [3, 64, 3]);
        assert_eq!(t.cmd(alice, "/mob sheep 3"), ["3 sheep spawned"]);
        assert_eq!(t.cmd(alice, "/spawnmob cow 50"), ["Mob quantity limited to server limit.", "10 cows spawned"]);
        t.host.entities.clear();
        assert_eq!(t.cmd(alice, "/spawnmob skeleton,spider"), ["2 skeletons spawned"]);
        assert_eq!(t.host.entities.len(), 1);
        let snbt = t.host.entities[0].2.clone().unwrap();
        assert!(snbt.contains("Passengers:[{id:\"minecraft:spider\""), "{snbt}");
        t.host.entities.clear();
        assert_eq!(t.cmd(alice, "/spawnmob sheep:red"), ["1 sheep spawned"]);
        assert!(t.host.entities[0].2.as_ref().unwrap().contains("Color:14b"));
        assert_eq!(t.cmd(alice, "/spawnmob zombie:baby 2 Bobby"), ["2 zombies spawned"]);
        let bob_at = t.host.p(bob).loc.block();
        assert_eq!(t.host.entities.last().unwrap().1.block(), bob_at);
        assert!(t.host.entities.last().unwrap().2.as_ref().unwrap().contains("IsBaby:1b"));
        assert_eq!(t.cmd(alice, "/spawnmob dragonfly"), ["Error: Invalid mob type."]);
        let lines = t.cmd(alice, "/spawnmob");
        assert_eq!(lines[0], "Command Help: /spawnmob");
        assert!(lines.last().unwrap().starts_with("Mobs: "));
        assert!(lines.last().unwrap().contains("zombie"));
        assert_eq!(t.console("spawnmob pig 1 Bobby"), ["1 pigs spawned"]);
        assert_eq!(t.console("spawnmob pig")[0], "Command Help: /spawnmob");
    }

    #[test]
    fn spawnmob_permissions_and_protect() {
        let (mut t, carol) = with_player(&["essentials.spawnmob", "essentials.spawnmob.cow"], "protect:\n  prevent:\n    spawn:\n      pig: true");
        t.host.p(carol).target = Some([3, 63, 3]);
        assert_eq!(t.cmd(carol, "/spawnmob cow"), ["1 cows spawned"]);
        assert_eq!(t.cmd(carol, "/spawnmob zombie"), ["Error: You don't have permission to spawn this mob."]);
        assert_eq!(t.cmd(carol, "/spawnmob cow,cow"), ["Error: You do not have permission to stack multiple mobs."]);
        assert_eq!(t.cmd(carol, "/spawnmob pig"), ["Error: Spawning this mob was disabled in the config file."]);
    }

    #[test]
    fn spawner_changes_type() {
        let (mut t, alice, _bob) = server();
        t.host.p(alice).target = Some([2, 63, 2]);
        assert_eq!(t.cmd(alice, "/spawner zombie"), ["Error: Target block must be a mob spawner."]);
        t.host.blocks.insert(("minecraft:overworld".into(), [2, 63, 2]), "minecraft:spawner".into());
        assert_eq!(t.cmd(alice, "/spawner zombie"), ["Changed spawner type to Zombie."]);
        assert_eq!(
            t.host.console_commands.last().unwrap(),
            "execute in minecraft:overworld run data merge block 2 63 2 {SpawnData:{entity:{id:\"minecraft:zombie\"}},SpawnPotentials:[],Delay:0s}"
        );
        assert_eq!(t.cmd(alice, "/mobspawner cavespider 40"), ["Changed spawner type to CaveSpider."]);
        assert!(t.host.console_commands.last().unwrap().contains("MinSpawnDelay:40s,MaxSpawnDelay:40s,Delay:40s"));
        assert_eq!(t.cmd(alice, "/spawner unicorn"), ["Error: Invalid mob type."]);
        assert_eq!(t.cmd(alice, "/spawner z")[0], "Command Help: /spawner");
        let (mut t, carol) = with_player(&["essentials.spawner"], "");
        t.host.p(carol).target = Some([2, 63, 2]);
        t.host.blocks.insert(("minecraft:overworld".into(), [2, 63, 2]), "minecraft:spawner".into());
        assert_eq!(t.cmd(carol, "/spawner zombie"), ["Error: You don't have permission to spawn this mob."]);
    }

    #[test]
    fn remove_by_kind_radius_and_world() {
        let (mut t, alice, _bob) = server();
        let at = |x: f64| Loc::new("minecraft:overworld", x, 64.0, 0.5);
        for (kind, x) in [("minecraft:zombie", 3.0), ("minecraft:cow", 5.0), ("minecraft:item", 6.0), ("minecraft:zombie", 400.0), ("minecraft:arrow", 7.0)] {
            t.host.entities.push((kind.into(), at(x), None));
        }
        t.host.entities.push(("minecraft:creeper".into(), Loc::new("minecraft:the_nether", 0.0, 64.0, 0.0), None));
        assert_eq!(t.cmd(alice, "/remove monsters 50"), ["Removed 1 entities."]);
        assert_eq!(t.cmd(alice, "/remove drops"), ["Removed 1 entities."]);
        assert_eq!(t.cmd(alice, "/remove arrow"), ["Removed 1 entities."]);
        assert_eq!(t.cmd(alice, "/remove hostile world_nether"), ["Removed 1 entities."]);
        assert_eq!(t.cmd(alice, "/remove zombie,cow"), ["Removed 2 entities."]);
        assert_eq!(t.cmd(alice, "/remove unicorns"), ["Invalid mob type.", "Removed 0 entities."]);
        assert_eq!(t.cmd(alice, "/remove all nowhere"), ["Error: Invalid world."]);
        assert_eq!(t.console("remove all world"), ["Removed 0 entities."]);
        assert_eq!(t.console("remove all")[0], "Command Help: /remove");
    }

    // ----- editsign ------------------------------------------------------------------------------

    #[test]
    fn editsign_set_clear_copy_paste() {
        let (mut t, alice, _bob) = server();
        let w = "minecraft:overworld".to_string();
        t.host.p(alice).target = Some([0, 64, 0]);
        assert_eq!(t.cmd(alice, "/editsign set 1 hi"), ["Error: You must be looking at a sign to edit its text."]);
        t.host.signs.insert((w.clone(), [0, 64, 0]), ["a".into(), "b".into(), "c".into(), "d".into()]);
        assert_eq!(t.cmd(alice, "/sign set 2 Hello &cthere"), ["Set line 2 to \"Hello there\"."]);
        assert_eq!(t.host.signs[&(w.clone(), [0, 64, 0])][1], "Hello §cthere");
        assert_eq!(t.cmd(alice, "/editsign clear 1"), ["Cleared line 1."]);
        assert_eq!(t.host.signs[&(w.clone(), [0, 64, 0])][0], "");
        assert_eq!(t.cmd(alice, "/editsign copy"), ["Sign copied! Paste it with /editsign paste."]);
        assert_eq!(t.cmd(alice, "/editsign clear"), ["Sign cleared."]);
        assert_eq!(t.host.signs[&(w.clone(), [0, 64, 0])], [String::new(), String::new(), String::new(), String::new()]);
        assert_eq!(t.cmd(alice, "/editsign paste"), ["Sign pasted!"]);
        assert_eq!(t.host.signs[&(w.clone(), [0, 64, 0])][1], "Hello §cthere");
        assert_eq!(t.cmd(alice, "/editsign copy 3"), ["Copied line 3 of sign! Paste it with /editsign paste 3."]);
        assert_eq!(t.cmd(alice, "/editsign paste 3"), ["Pasted line 3 of sign!"]);
        assert_eq!(t.cmd(alice, "/editsign set 5 x"), ["Error: You must enter a line number between 1-4."]);
        assert_eq!(t.cmd(alice, "/editsign set x y")[0], "Command Help: /editsign");
        assert_eq!(t.cmd(alice, "/editsign")[0], "Command Help: /editsign");
        assert_eq!(t.cmd(alice, "/editsign frob 1")[0], "Command Help: /editsign");
    }

    #[test]
    fn editsign_limit_and_colors_need_permission() {
        let (mut t, carol) = with_player(&["essentials.editsign"], "");
        t.host.p(carol).target = Some([0, 64, 0]);
        t.host.signs.insert(("minecraft:overworld".into(), [0, 64, 0]), Default::default());
        assert_eq!(t.cmd(carol, "/editsign set 1 abcdefghijklmnop"), ["Error: Your provided text is too big to fit on the target sign."]);
        assert_eq!(t.cmd(carol, "/editsign set 1 &chi"), ["Set line 1 to \"&chi\"."]);
    }

    // ----- fun ----------------------------------------------------------------------------------

    #[test]
    fn antioch_nuke_fireball_beezooka_kittycannon() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).target = Some([7, 63, 7]);
        assert!(t.cmd(alice, "/antioch").is_empty());
        assert_eq!(t.host.entities[0].0, "minecraft:tnt");
        assert_eq!(t.host.entities[0].1.block(), [7, 63, 7]);
        assert_eq!(t.cmd(alice, "/grenade go"), ["...lobbest thou thy Holy Hand Grenade of Antioch towards thy foe,", "who being naughty in My sight, shall snuff it."]);
        assert_eq!(t.said(bob).len(), 2);
        t.host.entities.clear();
        assert!(t.cmd(alice, "/nuke Bobby").is_empty());
        assert_eq!(t.said(bob), ["May death rain upon them."]);
        assert_eq!(t.host.entities.len(), 25);
        assert!(t.host.entities.iter().all(|e| e.0 == "minecraft:tnt" && e.1.y == 127.0));
        t.host.entities.clear();
        assert_eq!(t.console("nuke").len(), 0);
        assert_eq!(t.host.entities.len(), 50);
        t.host.entities.clear();
        t.cmd(alice, "/fireball");
        assert_eq!(t.host.entities[0].0, "minecraft:fireball");
        assert!(t.host.entities[0].2.as_ref().unwrap().contains("Motion:["));
        t.cmd(alice, "/fireball small 100");
        assert_eq!(t.host.entities[1].0, "minecraft:small_fireball");
        t.cmd(alice, "/fireball nonsense");
        assert_eq!(t.host.entities[2].0, "minecraft:fireball");
        t.host.entities.clear();
        t.cmd(alice, "/beezooka");
        assert_eq!(t.host.entities[0].0, "minecraft:bee");
        t.cmd(alice, "/kittycannon");
        assert_eq!(t.host.entities[1].0, "minecraft:cat");
        assert!(t.host.entities[1].2.as_ref().unwrap().contains("Age:-24000"));
        t.host.console_commands.clear();
        t.tick(20);
        assert_eq!(t.host.console_commands.iter().filter(|c| c.contains("minecraft:creeper")).count(), 2);
        assert_eq!(t.host.entities.len(), 0, "the projectiles are removed after exploding");
    }

    #[test]
    fn fireball_type_permission() {
        let (mut t, carol) = with_player(&["essentials.fireball", "essentials.fireball.fireball"], "");
        t.cmd(carol, "/fireball");
        assert_eq!(t.host.entities.len(), 1);
        assert_eq!(t.cmd(carol, "/fireball small"), ["Error: You do not have the essentials.fireball.small permission."]);
    }

    // ----- death policies -----------------------------------------------------------------------

    #[test]
    fn keepinv_keepxp_and_curse_policies() {
        let (mut t, carol) = with_player(&["essentials.keepinv", "essentials.keepxp"], "vanishing-items-policy: delete\nbinding-items-policy: drop");
        let mut vanishing = Item::new("diamond_sword", 1);
        vanishing.components = json!({"enchantments": {"minecraft:vanishing_curse": 1}});
        let mut binding = Item::new("diamond_helmet", 1);
        binding.components = json!({"enchantments": {"minecraft:binding_curse": 1}});
        t.host.p(carol).inventory[0] = Some(Item::new("stone", 5));
        t.host.p(carol).inventory[1] = Some(vanishing);
        t.host.p(carol).inventory[39] = Some(binding.clone());
        let d = crate::game::ess_world::death_policy(&mut t.ess, &mut t.host, carol);
        assert!(d.keep_inventory && d.keep_level);
        assert_eq!(d.drop, vec![binding]);
        let inv = t.host.p(carol).inventory.clone();
        assert!(inv[0].is_some() && inv[1].is_none() && inv[39].is_none());
        let (mut t, dave) = with_player(&[], "");
        let d = crate::game::ess_world::death_policy(&mut t.ess, &mut t.host, dave);
        assert!(!d.keep_inventory && !d.keep_level && d.drop.is_empty());
    }
}
