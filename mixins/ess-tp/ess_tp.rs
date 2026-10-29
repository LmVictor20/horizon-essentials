// horizon-essentials area `tp` (pure logic, standalone-tested): Teleport & places: tp*, tpa*, back, top, bottom, jump, tpr, settpr, home*, warp*, spawn, setspawn, world; warmups, cooldowns, safe teleports, /back on death.
// A port of EssentialsX's AsyncTeleport/AsyncTimedTeleport, LocationUtil, Warps, RandomTeleport, the home/tpa commands
// and EssentialsSpawn. Same message keys, config keys, permission nodes, userdata keys (homes.<name>, lastlocation,
// logoutlocation, teleportenabled, teleportauto, timestamps.lastteleport) and data files (warps/<name>.yml,
// spawn.yml, tpr.yml).

use crate::game::ess_core::*;
use crate::game::ess_text;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

const OVERWORLD: &str = "minecraft:overworld";
const NETHER: &str = "minecraft:the_nether";
const WARPS_PER_PAGE: usize = 20;

pub fn register(ess: &mut Ess) {
    let commands: [(&'static str, CmdFn, Option<TabFn>, &'static [(&'static str, &'static str)]); 32] = [
        ("back", cmd_back, Some(tab_players), &[("/<command>", "backCommandUsage1Description"), ("/<command> <player>", "backCommandUsage2Description")]),
        ("bottom", cmd_bottom, None, &[]),
        ("delhome", cmd_delhome, Some(tab_homes), &[("/<command> <name>", "delhomeCommandUsage1Description"), ("/<command> <player>:<name>", "delhomeCommandUsage2Description")]),
        ("home", cmd_home, Some(tab_homes), &[("/<command> <name>", "homeCommandUsage1Description"), ("/<command> <player>:<name>", "homeCommandUsage2Description")]),
        ("jump", cmd_jump, None, &[]),
        ("renamehome", cmd_renamehome, Some(tab_homes), &[("/<command> <name> <new name>", "renamehomeCommandUsage1Description"), ("/<command> <player>:<name> <new name>", "renamehomeCommandUsage2Description")]),
        ("sethome", cmd_sethome, None, &[("/<command> <name>", "sethomeCommandUsage1Description"), ("/<command> <player>:<name>", "sethomeCommandUsage2Description")]),
        ("settpr", cmd_settpr, None, &[("/<command> <world> center", "settprCommandUsage1Description"), ("/<command> <world> minrange <radius>", "settprCommandUsage2Description"), ("/<command> <world> maxrange <radius>", "settprCommandUsage3Description")]),
        ("setwarp", cmd_setwarp, None, &[("/<command> <warp>", "setwarpCommandUsage1Description")]),
        ("delwarp", cmd_delwarp, Some(tab_warps), &[("/<command> <warp>", "delwarpCommandUsage1Description")]),
        ("warp", cmd_warp, Some(tab_warps), &[("/<command> [page]", "warpCommandUsage1Description"), ("/<command> <warp> [player]", "warpCommandUsage2Description")]),
        ("warpinfo", cmd_warpinfo, Some(tab_warps), &[("/<command> <warp>", "warpinfoCommandUsage1Description")]),
        ("top", cmd_top, None, &[]),
        ("tp", cmd_tp, Some(tab_players), &[("/<command> <player>", "tpCommandUsage1Description"), ("/<command> <player> <other player>", "tpCommandUsage2Description")]),
        ("tpa", cmd_tpa, Some(tab_players), &[("/<command> <player>", "tpaCommandUsage1Description")]),
        ("tpaall", cmd_tpaall, Some(tab_players), &[("/<command> <player>", "tpaallCommandUsage1Description")]),
        ("tpacancel", cmd_tpacancel, Some(tab_players), &[("/<command>", "tpacancelCommandUsage1Description"), ("/<command> <player>", "tpacancelCommandUsage2Description")]),
        ("tpaccept", cmd_tpaccept, Some(tab_requests), &[("/<command>", "tpacceptCommandUsage1Description"), ("/<command> <player>", "tpacceptCommandUsage2Description"), ("/<command> *", "tpacceptCommandUsage3Description")]),
        ("tpahere", cmd_tpahere, Some(tab_players), &[("/<command> <player>", "tpahereCommandUsage1Description")]),
        ("tpall", cmd_tpall, Some(tab_players), &[("/<command> [player]", "tpallCommandUsage1Description")]),
        ("tpauto", cmd_tpauto, Some(tab_players), &[("/<command> [player]", "tpautoCommandUsage1Description")]),
        ("tpdeny", cmd_tpdeny, Some(tab_requests), &[("/<command>", "tpdenyCommandUsage1Description"), ("/<command> <player>", "tpdenyCommandUsage2Description"), ("/<command> *", "tpdenyCommandUsage3Description")]),
        ("tphere", cmd_tphere, Some(tab_players), &[("/<command> <player>", "tphereCommandUsage1Description")]),
        ("tpo", cmd_tpo, Some(tab_players), &[("/<command> <player>", "tpoCommandUsage1Description"), ("/<command> <player> <other player>", "tpoCommandUsage2Description")]),
        ("tpoffline", cmd_tpoffline, None, &[("/<command> <player>", "tpofflineCommandUsage1Description")]),
        ("tpohere", cmd_tpohere, Some(tab_players), &[("/<command> <player>", "tpohereCommandUsage1Description")]),
        ("tppos", cmd_tppos, None, &[("/<command> <x> <y> <z> [yaw] [pitch] [world]", "tpposCommandUsage1Description")]),
        ("tpr", cmd_tpr, None, &[("/<command>", "tprCommandUsage1Description"), ("/<command> <world>", "tprCommandUsage2Description"), ("/<command> <world> <player>", "tprCommandUsage3Description")]),
        ("tptoggle", cmd_tptoggle, Some(tab_players), &[("/<command> [player]", "tptoggleCommandUsage1Description")]),
        ("world", cmd_world, Some(tab_worlds), &[("/<command>", "worldCommandUsage1Description"), ("/<command> <world>", "worldCommandUsage2Description")]),
        ("setspawn", cmd_setspawn, None, &[]),
        ("spawn", cmd_spawn, Some(tab_players), &[]),
    ];
    for (name, run, tab, usages) in commands {
        ess.register(Command { name, run, tab, usages });
    }
    ess.listeners.tick.push(on_tick);
    ess.listeners.quit.push(on_quit);
    ess.listeners.join.push(on_join);
    ess.listeners.death.push(on_death);
    ess.listeners.respawn.push(on_respawn);
    ess.listeners.damage.push(on_damage);
}

// ===== Small helpers ===============================================================================

/// The Bukkit world name of a dimension (`world`, `world_nether`, `world_the_end`), as messages and
/// `essentials.worlds.<name>` nodes use it.
pub fn world_name(id: &str) -> String {
    match id {
        OVERWORLD => "world".into(),
        NETHER => "world_nether".into(),
        "minecraft:the_end" => "world_the_end".into(),
        other => other.strip_prefix("minecraft:").unwrap_or(other).to_string(),
    }
}

/// Essentials.getWorld: a world by index or by name.
fn find_world(host: &mut dyn Host, name: &str) -> Option<String> {
    let worlds = host.worlds();
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
        if let Some(w) = name.parse::<usize>().ok().and_then(|i| worlds.get(i)) {
            return Some(w.clone());
        }
    }
    let id = world_id(&name.to_lowercase());
    worlds.into_iter().find(|w| *w == id || world_name(w).eq_ignore_ascii_case(name))
}

fn num(n: i64) -> Arg {
    Arg::Plain(ess_text::group_digits(&n.to_string()))
}

/// (world name, block x, y, z), the arguments of `teleporting`, `homeSet`, `warpingTo`...
fn loc_args(l: &Loc) -> Vec<Arg> {
    let b = l.block();
    vec![Arg::Plain(world_name(&l.world)), num(b[0] as i64), num(b[1] as i64), num(b[2] as i64)]
}

/// Java's Math.round.
fn jround(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// String.split as Java does it: trailing empty strings dropped.
fn java_split(s: &str, sep: char) -> Vec<String> {
    let mut parts: Vec<String> = s.split(sep).map(str::to_string).collect();
    while parts.len() > 1 && parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

/// NumberUtil.isInt.
fn is_int(s: &str) -> bool {
    s.parse::<i32>().is_ok()
}

/// StringUtil.safeString: lower case, anything but a-z0-9 becomes `_`.
fn safe_string(s: &str) -> String {
    s.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { '_' }).collect()
}

/// StringUtil.sanitizeFileName: lower case, anything but a-z0-9- becomes `_`.
fn sanitize_file_name(s: &str) -> String {
    s.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { c } else { '_' }).collect()
}

fn is_reserved_file_name(s: &str) -> bool {
    let s = s.to_lowercase();
    matches!(s.as_str(), "con" | "prn" | "aux" | "nul")
        || ((s.starts_with("com") || s.starts_with("lpt")) && s.len() == 4 && s.as_bytes()[3].is_ascii_digit())
}

/// A player's display name. TODO(social): nicknames and prefixes once the social area exposes them.
fn display(ess: &mut Ess, host: &mut dyn Host, p: Uuid) -> String {
    host.name(p).or_else(|| ess.users.get(p).and_then(|u| u.name())).unwrap_or_else(|| uuid_string(p))
}

fn has_p(ess: &mut Ess, host: &mut dyn Host, p: Uuid, node: &str) -> bool {
    ess.has(host, Sender::Player(p), node)
}

fn online(host: &mut dyn Host, p: Uuid) -> bool {
    host.online().contains(&p)
}

fn user_loc(ess: &mut Ess, p: Uuid, key: &str) -> Option<Loc> {
    ess.users.get(p).and_then(|u| u.get(key).and_then(Loc::from_yaml))
}

fn tp_enabled(ess: &mut Ess, p: Uuid) -> bool {
    ess.users.get(p).and_then(|u| u.get("teleportenabled").and_then(Value::as_bool)).unwrap_or(true)
}

fn auto_tp(ess: &mut Ess, p: Uuid) -> bool {
    ess.users.get(p).is_some_and(|u| u.bool("teleportauto"))
}

fn god(ess: &mut Ess, p: Uuid) -> bool {
    ess.users.get(p).is_some_and(|u| u.bool("godmode"))
}

fn set_last_location(ess: &mut Ess, p: Uuid, at: &Loc) {
    ess.users.user(p).set("lastlocation", at.to_yaml());
}

/// The world spawn Essentials falls back on (the first normal world's).
fn main_spawn(host: &mut dyn Host) -> Loc {
    host.world_spawn(OVERWORLD).unwrap_or_else(|| Loc::new(OVERWORLD, 0.5, 64.0, 0.5))
}

fn world_spawn(host: &mut dyn Host, world: &str) -> Loc {
    host.world_spawn(world).unwrap_or_else(|| main_spawn(host))
}

/// The usage error when an argument is missing (EssentialsCommand.getPlayer with too few args).
fn player_arg(ctx: &mut Ctx, args: &[String], i: usize) -> Result<Uuid, CmdError> {
    match args.get(i) {
        Some(a) => ctx.online_player(a),
        None => Err(CmdError::Usage(None)),
    }
}

/// Whether `world-teleport-permissions` forbids `who` going from one world to another.
fn world_denied(ess: &mut Ess, host: &mut dyn Host, who: Sender, from: &str, to: &str) -> Result<(), CmdError> {
    if from != to && ess.config_bool("world-teleport-permissions", false) {
        let node = format!("essentials.worlds.{}", world_name(to));
        if !ess.has(host, who, &node) {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain(node)]));
        }
    }
    Ok(())
}

/// DateUtil.formatDateDiff for a duration from now (seconds to days; "now" when nothing is left).
fn date_diff(ess: &Ess, ms: i64) -> String {
    if ms == 0 {
        return ess.tl("now", &[]);
    }
    let mut left = ms.abs() + 50;
    let mut parts = Vec::new();
    for (size, one, many) in [(86_400_000, "day", "days"), (3_600_000, "hour", "hours"), (60_000, "minute", "minutes"), (1000, "second", "seconds")] {
        if parts.len() > 2 {
            break;
        }
        let d = left / size;
        if d > 0 {
            left -= d * size;
            parts.push(format!("{d} {}", ess.tl(if d > 1 { many } else { one }, &[])));
        }
    }
    if parts.is_empty() { ess.tl("now", &[]) } else { parts.join(" ") }
}

/// The translated text of an error (for `cooldownWithMessage`).
fn error_text(ess: &Ess, e: &CmdError) -> String {
    match e {
        CmdError::Tl(key, args) => ess.tl(key, args),
        CmdError::Msg(m) => ess_text::escape_tags(m),
        CmdError::PlayerNotFound => ess.tl("playerNotFound", &[]),
        _ => String::new(),
    }
}

/// Essentials.showError: `errorWithMessage` around the error.
fn show_error(ess: &mut Ess, host: &mut dyn Host, to: Sender, e: &CmdError) {
    match e {
        CmdError::Quiet | CmdError::Usage(_) => {}
        _ => {
            let text = error_text(ess, e);
            ess.send_tl(host, to, "errorWithMessage", &[Arg::Parsed(text)]);
        }
    }
}

// ===== State =======================================================================================

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum TpType {
    #[default]
    Normal,
    Back,
    Tpa,
}

/// A message sent when a teleport completes (the futures' thenAccept in EssentialsX).
#[derive(Clone)]
struct Note(Sender, &'static str, Vec<Arg>);

#[derive(Clone)]
enum Target {
    Loc(Loc),
    Player(Uuid),
    /// The bed, else wherever a respawn would put the player (AsyncTeleport.respawn).
    Respawn,
}

/// One teleport: who owns it (warmup, cooldown and messages), who moves, where, and what to say.
#[derive(Clone)]
struct Job {
    owner: Uuid,
    teleportee: Uuid,
    target: Target,
    ok: Vec<Note>,
    /// Where an error is shown (getNewExceptionFuture's sender).
    err_to: Option<Sender>,
    /// Sent on failure besides the error.
    err: Vec<Note>,
}

impl Job {
    fn new(owner: Uuid, teleportee: Uuid, target: Target, err_to: Sender) -> Job {
        Job { owner, teleportee, target, ok: Vec::new(), err_to: Some(err_to), err: Vec::new() }
    }
    fn ok(mut self, to: Uuid, key: &'static str, args: Vec<Arg>) -> Job {
        self.ok.push(Note(Sender::Player(to), key, args));
        self
    }
}

/// A warmup in progress (AsyncTimedTeleport).
struct Timer {
    job: Job,
    started: i64,
    delay: i64,
    init: [i64; 3],
    health: f64,
    can_move: bool,
    next: u64,
}

#[derive(Clone)]
struct Request {
    name: String,
    uuid: Uuid,
    here: bool,
    loc: Loc,
    time: i64,
}

#[derive(Default)]
struct TpState {
    /// By owner: one warmup per player.
    timers: HashMap<Uuid, Timer>,
    /// By target: tpa requests, oldest first (EssentialsX's LinkedHashMap by requester name).
    requests: HashMap<Uuid, Vec<Request>>,
    /// Until when a player may not be hurt by players (teleport-invulnerability).
    invulnerable: HashMap<Uuid, i64>,
    /// /sethome overwrite confirmation: (home, when).
    confirm: HashMap<Uuid, (String, i64)>,
    tp_type: HashMap<Uuid, TpType>,
    jump_lock: HashSet<Uuid>,
    /// Players seen dead (Host::is_dead) last tick.
    dead: HashSet<Uuid>,
    seed: u64,
}

fn st(ess: &mut Ess) -> &mut TpState {
    ess.state::<TpState>()
}

/// A random double in [0, 1) (xorshift64*, seeded from the clock).
fn random(ess: &mut Ess, host: &mut dyn Host) -> f64 {
    let s = st(ess);
    if s.seed == 0 {
        s.seed = (host.now_millis() as u64) ^ 0x9e37_79b9_7f4a_7c15;
    }
    s.seed ^= s.seed >> 12;
    s.seed ^= s.seed << 25;
    s.seed ^= s.seed >> 27;
    (s.seed.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
}

// ===== Teleport engine (AsyncTeleport / AsyncTimedTeleport) ========================================

fn deliver(ess: &mut Ess, host: &mut dyn Host, notes: &[Note]) {
    for Note(to, key, args) in notes {
        if let Sender::Player(p) = to {
            if !online(host, *p) {
                continue;
            }
        }
        ess.send_tl(host, *to, key, args);
    }
}

fn fail(ess: &mut Ess, host: &mut dyn Host, job: &Job, e: &CmdError) {
    if let Some(to) = job.err_to {
        show_error(ess, host, to, e);
    }
    deliver(ess, host, &job.err);
}

fn cooldown_applies(ess: &mut Ess, host: &mut dyn Host, owner: Uuid) -> bool {
    let bypass = has_p(ess, host, owner, "essentials.teleport.cooldown.bypass");
    match st(ess).tp_type.get(&owner).copied().unwrap_or_default() {
        TpType::Normal => !bypass,
        TpType::Back => !(bypass && has_p(ess, host, owner, "essentials.teleport.cooldown.bypass.back")),
        TpType::Tpa => !(bypass && has_p(ess, host, owner, "essentials.teleport.cooldown.bypass.tpa")),
    }
}

/// AsyncTeleport.cooldown: Err while teleport-cooldown runs; records the teleport unless `check`.
fn cooldown(ess: &mut Ess, host: &mut dyn Host, owner: Uuid, check: bool) -> Result<(), CmdError> {
    let now = host.now_millis();
    let last = ess.users.get(owner).and_then(|u| u.i64("timestamps.lastteleport")).unwrap_or(0);
    if last > 0 {
        let cd_ms = (ess.config_f64("teleport-cooldown", 0.0) * 1000.0) as i64;
        if last > now {
            ess.users.user(owner).set("timestamps.lastteleport", json!(now));
            return Ok(());
        }
        if last > now - cd_ms && cooldown_applies(ess, host, owner) {
            return Err(CmdError::Tl("timeBeforeTeleport", vec![Arg::Plain(date_diff(ess, last + cd_ms - now))]));
        }
    }
    if !check {
        ess.users.user(owner).set("timestamps.lastteleport", json!(now));
    }
    Ok(())
}

fn cancel_timer(ess: &mut Ess, host: &mut dyn Host, owner: Uuid, notify: bool) {
    let Some(timer) = st(ess).timers.remove(&owner) else { return };
    if notify {
        ess.send_tl(host, Sender::Player(owner), "pendingTeleportCancelled", &[]);
        if timer.job.teleportee != owner {
            ess.send_tl(host, Sender::Player(timer.job.teleportee), "pendingTeleportCancelled", &[]);
        }
    }
}

fn moved_key(l: &Loc) -> [i64; 3] {
    [(l.x * 0.3).round() as i64, (l.y * 0.3).round() as i64, (l.z * 0.3).round() as i64]
}

/// AsyncTeleport.teleport / teleportOther: cooldown, then a warmup (teleport-delay) unless bypassed.
/// `by` is who asked for it (None: the console, which never waits).
fn teleport(ess: &mut Ess, host: &mut dyn Host, job: Job, by: Option<Uuid>) {
    let delay = ess.config_f64("teleport-delay", 0.0);
    if let Err(e) = cooldown(ess, host, job.owner, true) {
        fail(ess, host, &job, &e);
        return;
    }
    let bypass = match by {
        None => true,
        Some(b) => [b, job.owner, job.teleportee].into_iter().any(|p| has_p(ess, host, p, "essentials.teleport.timer.bypass")),
    };
    if delay <= 0.0 || bypass {
        if let Err(e) = cooldown(ess, host, job.owner, false) {
            fail(ess, host, &job, &e);
            return;
        }
        finish(ess, host, job);
        return;
    }
    cancel_timer(ess, host, job.owner, false);
    let delay_ms = (delay * 1000.0) as i64;
    let warn = date_diff(ess, delay_ms);
    ess.send_tl(host, Sender::Player(job.teleportee), "dontMoveMessage", &[Arg::Plain(warn)]);
    let Some(at) = host.location(job.teleportee) else { return };
    let timer = Timer {
        started: host.now_millis(),
        delay: delay_ms,
        init: moved_key(&at),
        health: host.health(job.teleportee).0,
        can_move: has_p(ess, host, job.owner, "essentials.teleport.timer.move"),
        next: ess.scheduler.tick + 20,
        job,
    };
    st(ess).timers.insert(timer.job.owner, timer);
}

/// AsyncTeleport.now: no warmup; the cooldown only when asked.
fn now(ess: &mut Ess, host: &mut dyn Host, job: Job, with_cooldown: bool) {
    if with_cooldown {
        if let Err(e) = cooldown(ess, host, job.owner, false) {
            fail(ess, host, &job, &e);
            return;
        }
    }
    finish(ess, host, job);
}

fn finish(ess: &mut Ess, host: &mut dyn Host, job: Job) {
    match now_async(ess, host, job.owner, job.teleportee, &job.target) {
        Ok(_) => deliver(ess, host, &job.ok),
        Err(e) => fail(ess, host, &job, &e),
    }
}

/// AsyncTeleport.nowAsync: /back point, safety, centering, the move, invulnerability.
fn now_async(ess: &mut Ess, host: &mut dyn Host, owner: Uuid, teleportee: Uuid, target: &Target) -> Result<Loc, CmdError> {
    cancel_timer(ess, host, owner, false);
    let loc = match target {
        Target::Loc(l) => l.clone(),
        Target::Player(p) => host.location(*p).ok_or(CmdError::PlayerNotFound)?,
        Target::Respawn => match host.bed_spawn(teleportee) {
            Some(bed) => bed,
            None => ess.on_respawn(host, teleportee).unwrap_or_else(|| {
                let world = host.location(teleportee).map_or_else(|| OVERWORLD.to_string(), |l| l.world);
                world_spawn(host, &world)
            }),
        },
    };
    if has_p(ess, host, teleportee, "essentials.back.onteleport") {
        if let Some(here) = host.location(teleportee) {
            set_last_location(ess, teleportee, &here);
        }
    }
    let safety = ess.config_bool("teleport-safety", true);
    let force_disable = ess.config_bool("force-disable-teleport-safety", false);
    let center = ess.config_bool("teleport-to-center", true);
    // A destination in an unloaded chunk can't be inspected: it is used as given (centered).
    let known = host.block(&loc.world, loc.block()).is_some();
    let dest = if known && unsafe_for_user(ess, host, teleportee, &loc) {
        if !safety {
            return Err(CmdError::Tl("unsafeTeleportDestination", loc_args(&loc)));
        }
        if force_disable { loc } else { safe_destination_for_user(ess, host, teleportee, &loc)? }
    } else if force_disable || !center {
        loc
    } else {
        rounded(&loc)
    };
    if !host.teleport(teleportee, &dest) {
        return Err(CmdError::Tl("noLocationFound", vec![]));
    }
    after_teleport(ess, host, teleportee);
    Ok(dest)
}

/// EssentialsPlayerListener.onPlayerTeleport: teleport-invulnerability.
fn after_teleport(ess: &mut Ess, host: &mut dyn Host, p: Uuid) {
    let secs = ess.config_i64("teleport-invulnerability", 0);
    if secs <= 0 {
        return;
    }
    let until = host.now_millis() + secs * 1000;
    st(ess).invulnerable.insert(p, until);
    host.set_invulnerable(p, true);
    ess.scheduler.later(
        secs as u64 * 20 + 1,
        Box::new(move |ess, host| {
            let now = host.now_millis();
            if st(ess).invulnerable.get(&p).is_some_and(|u| *u < now) {
                st(ess).invulnerable.remove(&p);
                if !god(ess, p) {
                    host.set_invulnerable(p, false);
                }
            }
        }),
    );
}

/// Deaths and respawns, seen as Host::is_dead changing: `on_death` at the death point, then, when the player
/// is back, `on_respawn`'s answer moves them (PlayerRespawnEvent.setRespawnLocation).
fn poll_deaths(ess: &mut Ess, host: &mut dyn Host) {
    for p in host.online() {
        let dead = host.is_dead(p);
        let was = st(ess).dead.contains(&p);
        if dead && !was {
            st(ess).dead.insert(p);
            if let Some(at) = host.location(p) {
                ess.on_death(host, p, &at);
            }
        } else if !dead && was {
            st(ess).dead.remove(&p);
            if let Some(at) = ess.on_respawn(host, p) {
                host.teleport(p, &at);
            }
        }
    }
}

fn on_tick(ess: &mut Ess, host: &mut dyn Host) {
    poll_deaths(ess, host);
    let tick = ess.scheduler.tick;
    let due: Vec<Uuid> = st(ess).timers.iter().filter(|(_, t)| t.next <= tick).map(|(o, _)| *o).collect();
    for owner in due {
        let Some(mut timer) = st(ess).timers.remove(&owner) else { continue };
        timer.next = tick + 20;
        let teleportee = timer.job.teleportee;
        if !online(host, owner) || !online(host, teleportee) {
            continue;
        }
        let Some(at) = host.location(teleportee) else { continue };
        let health = host.health(teleportee).0;
        if !timer.can_move && (moved_key(&at) != timer.init || health < timer.health) {
            st(ess).timers.insert(owner, timer);
            cancel_timer(ess, host, owner, true);
            continue;
        }
        timer.health = health;
        if host.now_millis() > timer.started + timer.delay {
            if let Err(e) = cooldown(ess, host, owner, false) {
                let text = error_text(ess, &e);
                ess.send_tl(host, Sender::Player(owner), "cooldownWithMessage", &[Arg::Parsed(text.clone())]);
                if teleportee != owner {
                    ess.send_tl(host, Sender::Player(teleportee), "cooldownWithMessage", &[Arg::Parsed(text)]);
                }
            }
            ess.send_tl(host, Sender::Player(teleportee), "teleportationCommencing", &[]);
            finish(ess, host, timer.job);
        } else {
            st(ess).timers.insert(owner, timer);
        }
    }
}

fn on_quit(ess: &mut Ess, _host: &mut dyn Host, p: Uuid) {
    st(ess).timers.retain(|owner, t| *owner != p && t.job.teleportee != p);
    st(ess).dead.remove(&p);
}

fn on_damage(ess: &mut Ess, host: &mut dyn Host, p: Uuid, cause: &str, _amount: f64) -> Allow {
    // EssentialsX cancels player-vs-player damage during teleport-invulnerability.
    let cause = cause.strip_prefix("minecraft:").unwrap_or(cause);
    let by_player = matches!(cause, "player" | "player_attack" | "player_explosion");
    let now = host.now_millis();
    if by_player && st(ess).invulnerable.get(&p).is_some_and(|u| *u >= now) { Allow::No } else { Allow::Yes }
}

// ===== Safe locations (LocationUtil) ================================================================

fn block_id(host: &mut dyn Host, world: &str, pos: [i32; 3]) -> String {
    let state = host.block(world, pos).unwrap_or_else(|| "minecraft:air".into());
    let id = state.split('[').next().unwrap_or("").to_string();
    id.strip_prefix("minecraft:").map(str::to_string).unwrap_or(id)
}

/// Blocks a player can stand in (Bukkit's Material.isTransparent, minus barrier/path/farmland, plus light).
fn hollow(ess: &Ess, id: &str) -> bool {
    if matches!(id, "water" | "bubble_column") {
        return ess.config_bool("is-water-safe", false);
    }
    passable(id)
}

/// The blocks of `hollow` without water: a block id without the `minecraft:` namespace or state. Also what
/// /jump looks through (with water: LocationUtil.TRANSPARENT_MATERIALS).
pub fn passable(id: &str) -> bool {
    id.ends_with("air")
        || id == "light"
        || id.ends_with("_sapling")
        || id.ends_with("_button")
        || id.ends_with("_carpet")
        || id.ends_with("rail")
        || id.ends_with("torch")
        || id.ends_with("_stem")
        || id.ends_with("_tulip")
        || matches!(
            id,
            "short_grass" | "grass" | "tall_grass" | "fern" | "large_fern" | "dead_bush" | "dandelion" | "poppy"
                | "blue_orchid" | "allium" | "azure_bluet" | "oxeye_daisy" | "cornflower" | "lily_of_the_valley"
                | "wither_rose" | "sunflower" | "lilac" | "rose_bush" | "peony" | "torchflower" | "brown_mushroom"
                | "red_mushroom" | "fire" | "soul_fire" | "redstone_wire" | "wheat" | "carrots" | "potatoes"
                | "beetroots" | "lever" | "snow" | "sugar_cane" | "nether_portal" | "end_portal" | "repeater"
                | "comparator" | "vine" | "nether_wart" | "cocoa" | "tripwire" | "tripwire_hook" | "flower_pot"
                | "structure_void" | "end_gateway"
        )
}

fn damaging(ess: &Ess, host: &mut dyn Host, w: &str, x: i32, y: i32, z: i32) -> bool {
    let below = block_id(host, w, [x, y - 1, z]);
    if matches!(below.as_str(), "cactus" | "campfire" | "fire" | "magma_block" | "soul_campfire" | "soul_fire" | "sweet_berry_bush" | "wither_rose" | "lava")
        || below.ends_with("_bed")
    {
        return true;
    }
    let block = block_id(host, w, [x, y, z]);
    if block == "nether_portal" {
        return true;
    }
    let above = block_id(host, w, [x, y + 1, z]);
    !hollow(ess, &block) || !hollow(ess, &above)
}

fn above_air(ess: &Ess, host: &mut dyn Host, w: &str, x: i32, y: i32, z: i32) -> bool {
    let (_, max) = host.height_range(w);
    y > max || hollow(ess, &block_id(host, w, [x, y - 1, z]))
}

fn unsafe_block(ess: &Ess, host: &mut dyn Host, w: &str, x: i32, y: i32, z: i32) -> bool {
    damaging(ess, host, w, x, y, z) || above_air(ess, host, w, x, y, z)
}

fn flyer(ess: &mut Ess, host: &mut dyn Host, p: Uuid) -> bool {
    matches!(host.game_mode(p), GameMode::Creative | GameMode::Spectator) || god(ess, p)
}

/// LocationUtil.isBlockUnsafeForUser (the world border is vanilla's ±29,999,984 and never in the way).
fn unsafe_for_user(ess: &mut Ess, host: &mut dyn Host, p: Uuid, loc: &Loc) -> bool {
    let same_world = host.location(p).is_some_and(|l| l.world == loc.world);
    if same_world && flyer(ess, host, p) && host.allow_flight(p) {
        return false;
    }
    let b = loc.block();
    unsafe_block(ess, host, &loc.world, b[0], b[1], b[2])
}

/// LocationUtil.getRoundedDestination.
fn rounded(l: &Loc) -> Loc {
    let b = l.block();
    Loc { world: l.world.clone(), x: b[0] as f64 + 0.5, y: jround(l.y) as f64, z: b[2] as f64 + 0.5, yaw: l.yaw, pitch: l.pitch }
}

fn logical_height(world: &str, min: i32, max: i32) -> i32 {
    match world {
        NETHER => 128,
        "minecraft:the_end" => 256,
        OVERWORLD => 384,
        _ => max - min,
    }
}

fn volume() -> Vec<(i32, i32, i32)> {
    let mut v = Vec::new();
    for x in -3..=3 {
        for y in -3..=3 {
            for z in -3..=3 {
                v.push((x, y, z));
            }
        }
    }
    v.sort_by_key(|(x, y, z)| x * x + y * y + z * z);
    v
}

fn should_fly(ess: &Ess, host: &mut dyn Host, loc: &Loc) -> bool {
    let (min, _) = host.height_range(&loc.world);
    let b = loc.block();
    let mut y = jround(loc.y);
    let mut count = 0;
    while unsafe_block(ess, host, &loc.world, b[0], y, b[2]) && y >= min {
        y -= 1;
        count += 1;
        if count > 2 {
            return true;
        }
    }
    y < min
}

fn safe_destination_for_user(ess: &mut Ess, host: &mut dyn Host, p: Uuid, loc: &Loc) -> Result<Loc, CmdError> {
    if !ess.config_bool("force-safe-teleport-location", false) && flyer(ess, host, p) {
        if should_fly(ess, host, loc) && host.allow_flight(p) {
            host.set_flying(p, true);
        }
        return Ok(if ess.config_bool("teleport-to-center", true) { rounded(loc) } else { loc.clone() });
    }
    safe_destination(ess, host, loc)
}

/// LocationUtil.getSafeDestination.
fn safe_destination(ess: &Ess, host: &mut dyn Host, loc: &Loc) -> Result<Loc, CmdError> {
    let w = loc.world.as_str();
    let (min_y, max_y) = host.height_range(w);
    let logical = logical_height(w, min_y, max_y);
    let world_max = if ess.config_bool("consider-world-height-for-teleport-safety", true) && loc.block()[1] < logical { logical } else { max_y };
    let mut x = loc.x.floor() as i32;
    let mut y = jround(loc.y);
    let mut z = loc.z.floor() as i32;
    let (ox, oy, oz) = (x, y, z);
    let unsafe_at =|host: &mut dyn Host, x: i32, y: i32, z: i32| y >= world_max || unsafe_block(ess, host, w, x, y, z);
    // EssentialsX stops at y 0 (from before 1.18 worlds went below it); the world's bottom is meant.
    while above_air(ess, host, w, x, y, z) {
        y -= 1;
        if y < min_y {
            y = oy;
            break;
        }
    }
    if unsafe_at(host, x, y, z) {
        x = if jround(loc.x) == ox { x - 1 } else { x + 1 };
        z = if jround(loc.z) == oz { z - 1 } else { z + 1 };
    }
    let vol = volume();
    let mut i = 0;
    while unsafe_at(host, x, y, z) {
        i += 1;
        if i >= vol.len() {
            x = ox;
            y = (oy + 3).clamp(min_y, world_max);
            z = oz;
            break;
        }
        x = ox + vol[i].0;
        y = (oy + vol[i].1).clamp(min_y, world_max);
        z = oz + vol[i].2;
    }
    while unsafe_at(host, x, y, z) {
        y += 1;
        if y >= world_max {
            x += 1;
            break;
        }
    }
    while unsafe_at(host, x, y, z) {
        y -= 1;
        if y <= min_y + 1 {
            x += 1;
            y = host.highest_y(w, x, z).map_or(world_max, |h| (h + 1).min(world_max));
            if x - 48 > loc.block()[0] {
                return Err(CmdError::Tl("holeInFloor", vec![]));
            }
        }
    }
    Ok(Loc { world: loc.world.clone(), x: x as f64 + 0.5, y: y as f64, z: z as f64 + 0.5, yaw: loc.yaw, pitch: loc.pitch })
}

// ===== Homes ========================================================================================

fn home_names(ess: &mut Ess, p: Uuid) -> Vec<String> {
    let mut names: Vec<String> = match ess.users.get(p).and_then(|u| u.get("homes")) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => Vec::new(),
    };
    names.sort();
    names
}

/// UserData.getHomeName: a number picks the n-th home.
fn home_key(ess: &mut Ess, p: Uuid, search: &str) -> String {
    if let Ok(n) = search.parse::<i32>() {
        if let Some(name) = usize::try_from(n - 1).ok().and_then(|i| home_names(ess, p).get(i).cloned()) {
            return name;
        }
    }
    search.to_string()
}

fn home_loc(ess: &mut Ess, p: Uuid, name: &str) -> Option<Loc> {
    let key = home_key(ess, p, name);
    ess.users.get(p).and_then(|u| u.get("homes").and_then(|h| h.get(&key)).and_then(Loc::from_yaml))
}

fn remove_home(ess: &mut Ess, p: Uuid, key: &str) -> Option<Value> {
    let user = ess.users.user(p);
    let removed = user.data.get_mut("homes").and_then(Value::as_object_mut).and_then(|m| m.remove(key));
    if removed.is_some() {
        user.dirty = true;
    }
    removed
}

/// UserData.getHome(Location): the first home in that world, else the first home.
fn home_near(ess: &mut Ess, p: Uuid, world: &str) -> Option<Loc> {
    let homes: Vec<Loc> = home_names(ess, p).iter().filter_map(|h| home_loc(ess, p, h)).collect();
    homes.iter().find(|l| l.world == world).or(homes.first()).cloned()
}

fn set_limit(ess: &Ess, set: &str) -> i64 {
    let default = ess.config_i64("sethome-multiple.default", 3);
    ess.config_i64(&format!("sethome-multiple.{set}"), default)
}

/// Settings.getHomeLimit.
fn home_limit(ess: &mut Ess, host: &mut dyn Host, p: Uuid) -> i64 {
    let mut limit = 1;
    if has_p(ess, host, p, "essentials.sethome.multiple") {
        limit = set_limit(ess, "default");
    }
    let sets: Vec<String> = match ess.config("sethome-multiple") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => Vec::new(),
    };
    for set in sets {
        if has_p(ess, host, p, &format!("essentials.sethome.multiple.{set}")) && limit < set_limit(ess, &set) {
            limit = set_limit(ess, &set);
        }
    }
    limit
}

fn cmd_home(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let mut player = user;
    let mut home = String::new();
    if let Some(arg) = args.first() {
        let parts = java_split(arg, ':');
        if parts[0].len() == arg.len() || !ctx.has("essentials.home.others") {
            home = parts[0].clone();
        } else {
            player = ctx.any_player(&parts[0])?;
            if parts.len() > 1 {
                home = parts[1].clone();
            }
        }
    }
    if home.eq_ignore_ascii_case("bed") {
        if !ctx.has("essentials.home.bed") {
            return Err(CmdError::Tl("noAccessCommand", vec![]));
        }
        if !online(ctx.host, player) {
            return Err(CmdError::Tl("bedOffline", vec![]));
        }
        let Some(bed) = ctx.host.bed_spawn(player) else { return Err(CmdError::Tl("bedMissing", vec![])) };
        let job = Job::new(user, user, Target::Loc(bed), ctx.sender).ok(user, "teleportHome", vec![Arg::from("bed")]);
        teleport(ctx.ess, ctx.host, job, Some(user));
        return Ok(());
    }
    match go_home(ctx, user, player, &home.to_lowercase()) {
        Err(CmdError::Usage(_)) => {}
        other => return other,
    }
    let homes = home_names(ctx.ess, player);
    let bed = if online(ctx.host, player) { ctx.host.bed_spawn(player) } else { None };
    if homes.is_empty() && player == user {
        if !ctx.ess.config_bool("spawn-if-no-home", true) {
            return Err(CmdError::Tl("noHomeSetPlayer", vec![]));
        }
        let target = if bed.is_some() && ctx.has("essentials.home.bed") {
            Target::Respawn
        } else {
            let world = ctx.host.location(user).map_or_else(|| OVERWORLD.to_string(), |l| l.world);
            Target::Loc(world_spawn(ctx.host, &world))
        };
        teleport(ctx.ess, ctx.host, Job::new(user, user, target, ctx.sender), Some(user));
        Ok(())
    } else if homes.is_empty() || !homes.iter().any(|h| home_loc(ctx.ess, player, h).is_some()) {
        Err(CmdError::Tl("noHomeSetPlayer", vec![]))
    } else if homes.len() == 1 && player == user {
        go_home(ctx, user, player, &homes[0])
    } else {
        let mut list: Vec<String> = homes.clone();
        if ctx.has("essentials.home.bed") {
            list.push(ctx.ess.tl(if bed.is_some() { "bed" } else { "bedNull" }, &[]));
        }
        let count = homes.len().to_string();
        let limit = if !online(ctx.host, player) {
            "?".to_string()
        } else if has_p(ctx.ess, ctx.host, player, "essentials.sethome.multiple.unlimited") {
            "*".to_string()
        } else {
            home_limit(ctx.ess, ctx.host, player).to_string()
        };
        ctx.tl("homes", &[Arg::Parsed(list.join(", ")), Arg::Plain(count), Arg::Plain(limit)]);
        Ok(())
    }
}

fn go_home(ctx: &mut Ctx, user: Uuid, player: Uuid, home: &str) -> CmdResult {
    if home.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let Some(loc) = home_loc(ctx.ess, player, home) else { return Err(CmdError::Usage(None)) };
    let here = ctx.host.location(user).map(|l| l.world).unwrap_or_default();
    if here != loc.world && ctx.ess.config_bool("world-home-permissions", false) {
        let node = format!("essentials.worlds.{}", world_name(&loc.world));
        if !ctx.has(&node) {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain(node)]));
        }
    }
    let job = Job::new(user, user, Target::Loc(loc), ctx.sender).ok(user, "teleportHome", vec![Arg::from(home)]);
    teleport(ctx.ess, ctx.host, job, Some(user));
    Ok(())
}

fn cmd_sethome(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let mut owner = user;
    let mut name = "home".to_string();
    if let Some(first) = args.first() {
        let parts = java_split(first, ':');
        let a: Vec<String> = if parts[0].len() != first.len() { parts } else { args.to_vec() };
        if a.len() < 2 {
            name = a[0].to_lowercase();
        } else {
            name = a[1].to_lowercase();
            if ctx.has("essentials.sethome.others") {
                owner = ctx.any_player(&a[0])?;
            }
        }
    }
    // checkHomeLimit
    if !ctx.has("essentials.sethome.multiple.unlimited") {
        let limit = home_limit(ctx.ess, ctx.host, user);
        let homes = home_names(ctx.ess, owner);
        if homes.len() as i64 >= limit {
            if !homes.contains(&name) {
                return Err(CmdError::Tl("maxHomes", vec![num(limit)]));
            }
        } else if limit == 1 {
            name = "home".into();
        }
    }
    if name == "bed" || is_int(&name) {
        return Err(CmdError::Tl("invalidHomeName", vec![]));
    }
    let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
    let safety = ctx.ess.config_bool("teleport-safety", true);
    let force_disable = ctx.ess.config_bool("force-disable-teleport-safety", false);
    if (!safety || !force_disable) && unsafe_for_user(ctx.ess, ctx.host, owner, &at) {
        return Err(CmdError::Tl("unsafeTeleportDestination", loc_args(&at)));
    }
    let key = safe_string(&name);
    let now = ctx.host.now_millis();
    if ctx.ess.config_bool("confirm-home-overwrite", false) && home_names(ctx.ess, owner).contains(&key) {
        let confirmed = st(ctx.ess).confirm.get(&owner).is_some_and(|(n, when)| *n == name && now - when <= 120_000);
        if !confirmed {
            st(ctx.ess).confirm.insert(owner, (name.clone(), now));
            ctx.tl("homeConfirmation", &[Arg::Plain(name)]);
            return Ok(());
        }
    }
    ctx.ess.users.user(owner).set(&format!("homes.{key}"), at.to_yaml());
    let mut homeset = loc_args(&at);
    homeset.push(Arg::Plain(name));
    ctx.tl("homeSet", &homeset);
    st(ctx.ess).confirm.remove(&owner);
    Ok(())
}

fn delete_home(ctx: &mut Ctx, player: Uuid, home: &str) {
    let mut key = home_key(ctx.ess, player, home);
    if !home_names(ctx.ess, player).contains(&key) {
        key = safe_string(&key);
    }
    if remove_home(ctx.ess, player, &key).is_some() {
        ctx.tl("deleteHome", &[Arg::from(home)]);
    } else {
        ctx.tl("invalidHome", &[Arg::from(home)]);
    }
}

fn cmd_delhome(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Some(first) = args.first() else { return Err(CmdError::Usage(None)) };
    let user = ctx.sender.player();
    let parts = java_split(first, ':');
    let expanded: Vec<String> = if parts[0].len() != first.len() { parts } else { args.to_vec() };
    let (player, name) = if expanded.len() > 1 && (user.is_none() || ctx.has("essentials.delhome.others")) {
        (ctx.any_player(&expanded[0])?, expanded[1].to_lowercase())
    } else if let Some(u) = user {
        (u, expanded[0].to_lowercase())
    } else {
        return Err(CmdError::Usage(None));
    };
    match name.as_str() {
        "bed" => return Err(CmdError::Tl("invalidHomeName", vec![])),
        "*" => {
            for home in home_names(ctx.ess, player) {
                delete_home(ctx, player, &home);
            }
        }
        _ => delete_home(ctx, player, &name),
    }
    Ok(())
}

fn cmd_renamehome(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let mut owner = user;
    let (old, new) = match args.len() {
        2 => {
            let parts: Vec<&str> = args[0].splitn(2, ':').collect();
            let new = args[1].to_lowercase();
            if parts.len() == 2 {
                if ctx.has("essentials.renamehome.others") {
                    owner = ctx.any_player(parts[0])?;
                }
                (parts[1].to_lowercase(), new)
            } else {
                (args[0].to_lowercase(), new)
            }
        }
        3 => {
            if !ctx.has("essentials.renamehome.others") {
                return Err(CmdError::Usage(None));
            }
            owner = ctx.any_player(&args[0])?;
            (args[1].to_lowercase(), args[2].to_lowercase())
        }
        _ => return Err(CmdError::Usage(None)),
    };
    if [&old, &new].iter().any(|n| n.as_str() == "bed" || is_int(n)) {
        return Err(CmdError::Tl("invalidHomeName", vec![]));
    }
    let Some(loc) = remove_home(ctx.ess, owner, &old) else { return Err(CmdError::Tl("invalidHome", vec![Arg::Plain(old)])) };
    ctx.ess.users.user(owner).set(&format!("homes.{}", safe_string(&new)), loc);
    ctx.tl("homeRenamed", &[Arg::Plain(old), Arg::Plain(new)]);
    st(ctx.ess).confirm.remove(&owner);
    Ok(())
}

// ===== /back, /top, /bottom, /jump ===================================================================

fn cmd_back(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let others = !args.is_empty() && ctx.has("essentials.back.others");
    match ctx.sender {
        Sender::Player(user) if !others => teleport_back(ctx, user),
        Sender::Console if args.is_empty() => Err(CmdError::Usage(None)),
        _ => {
            let target = ctx.online_player(&args[0])?;
            let name = ctx.host.name(target).unwrap_or_default();
            ctx.tl("backOther", &[Arg::Plain(name)]);
            teleport_back(ctx, target)
        }
    }
}

fn teleport_back(ctx: &mut Ctx, user: Uuid) -> CmdResult {
    let Some(last) = user_loc(ctx.ess, user, "lastlocation") else { return Err(CmdError::Tl("noLocationFound", vec![])) };
    let last_world = world_name(&last.world);
    let requester = ctx.sender.player();
    if requester.is_some() {
        let here = ctx.host.location(user).map(|l| l.world).unwrap_or_default();
        if here != last.world && ctx.ess.config_bool("world-teleport-permissions", false) {
            let node = format!("essentials.worlds.{last_world}");
            if !has_p(ctx.ess, ctx.host, user, &node) {
                return Err(CmdError::Tl("noPerm", vec![Arg::Plain(node)]));
            }
        }
        let node = format!("essentials.back.into.{last_world}");
        if !ctx.has(&node) {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain(node)]));
        }
    }
    st(ctx.ess).tp_type.insert(user, TpType::Back);
    ctx.tl_to(Sender::Player(user), "backUsageMsg", &loc_args(&last));
    teleport(ctx.ess, ctx.host, Job::new(user, user, Target::Loc(last), ctx.sender), requester);
    Ok(())
}

fn cmd_top(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    vertical(ctx, true)
}

fn cmd_bottom(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    vertical(ctx, false)
}

fn vertical(ctx: &mut Ctx, top: bool) -> CmdResult {
    let user = ctx.player()?;
    let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
    let (min, max) = ctx.host.height_range(&at.world);
    let b = at.block();
    let start = Loc { world: at.world.clone(), x: b[0] as f64, y: if top { max } else { min } as f64, z: b[2] as f64, yaw: at.yaw, pitch: at.pitch };
    let safe = safe_destination(ctx.ess, ctx.host, &start)?;
    let key = if top { "teleportTop" } else { "teleportBottom" };
    let job = Job::new(user, user, Target::Loc(safe.clone()), ctx.sender).ok(user, key, loc_args(&safe));
    teleport(ctx.ess, ctx.host, job, Some(user));
    Ok(())
}

fn cmd_jump(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    if args.first().is_some_and(|a| a.contains("lock")) && ctx.has("essentials.jump.lock") {
        // TODO(items): the fly-click-jump itself (right click with a compass) belongs with powertools.
        let s = st(ctx.ess);
        let enabled = !s.jump_lock.remove(&user);
        if enabled {
            s.jump_lock.insert(user);
        }
        ctx.tl(if enabled { "jumpEasterEnable" } else { "jumpEasterDisable" }, &[]);
        return Ok(());
    }
    let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
    let Some(target) = ctx.host.target_block(user, 300.0) else { return Err(CmdError::Tl("jumpError", vec![])) };
    let loc = Loc { world: at.world, x: target[0] as f64, y: target[1] as f64 + 1.0, z: target[2] as f64, yaw: at.yaw, pitch: at.pitch };
    teleport(ctx.ess, ctx.host, Job::new(user, user, Target::Loc(loc), ctx.sender), Some(user));
    Ok(())
}

// ===== /tp family =====================================================================================

fn coord(base: f64, arg: &str) -> Result<f64, CmdError> {
    let parse = |s: &str| s.parse::<f64>().map_err(|_| CmdError::Msg(format!("For input string: \"{s}\"")));
    match arg.strip_prefix('~') {
        Some("") => Ok(base),
        Some(rel) => Ok(base + parse(rel)?),
        None => parse(arg),
    }
}

fn coords(at: &Loc, args: &[String]) -> Result<Loc, CmdError> {
    let x = coord(at.x, &args[0])?;
    let y = coord(at.y, &args[1])?;
    let z = coord(at.z, &args[2])?;
    Ok(Loc { world: at.world.clone(), x, y, z, yaw: at.yaw, pitch: at.pitch })
}

fn check_bounds(ctx: &mut Ctx, l: &Loc) -> CmdResult {
    if [l.x, l.y, l.z].iter().any(|v| v.abs() > 30_000_000.0) {
        return Err(CmdError::Usage(Some(ctx.ess.tl("teleportInvalidLocation", &[]))));
    }
    Ok(())
}

fn world_of(host: &mut dyn Host, p: Uuid) -> String {
    host.location(p).map(|l| l.world).unwrap_or_default()
}

fn teleport_disabled(ctx: &mut Ctx, p: Uuid) -> CmdResult {
    if !tp_enabled(ctx.ess, p) {
        let name = display(ctx.ess, ctx.host, p);
        return Err(CmdError::Tl("teleportDisabled", vec![Arg::Plain(name)]));
    }
    Ok(())
}

fn cmd_tp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(user) = ctx.sender else { return tp_console(ctx, args) };
    let me = Sender::Player(user);
    match args.len() {
        0 => Err(CmdError::Usage(None)),
        1 => {
            let target = ctx.any_player(&args[0])?;
            teleport_disabled(ctx, target)?;
            if !online(ctx.host, target) {
                if ctx.has("essentials.tpoffline") {
                    let name = display(ctx.ess, ctx.host, target);
                    return Err(CmdError::Tl("teleportOffline", vec![Arg::Plain(name)]));
                }
                return Err(CmdError::PlayerNotFound);
            }
            let (from, to) = (world_of(ctx.host, user), world_of(ctx.host, target));
            world_denied(ctx.ess, ctx.host, me, &from, &to)?;
            let name = display(ctx.ess, ctx.host, target);
            ctx.tl("teleportToPlayer", &[Arg::Plain(name)]);
            teleport(ctx.ess, ctx.host, Job::new(user, user, Target::Player(target), me), Some(user));
            Ok(())
        }
        3 => {
            if !ctx.has("essentials.tp.position") {
                return Err(CmdError::Tl("noPerm", vec![Arg::from("essentials.tp.position")]));
            }
            let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
            let loc = coords(&at, args)?;
            check_bounds(ctx, &loc)?;
            let job = Job::new(user, user, Target::Loc(loc.clone()), me).ok(user, "teleporting", loc_args(&loc));
            now(ctx.ess, ctx.host, job, true);
            Ok(())
        }
        4 => {
            if !ctx.has("essentials.tp.others") {
                return Err(CmdError::Tl("noPerm", vec![Arg::from("essentials.tp.others")]));
            }
            if !ctx.has("essentials.tp.position") {
                return Err(CmdError::Tl("noPerm", vec![Arg::from("essentials.tp.position")]));
            }
            let target = player_arg(ctx, args, 0)?;
            let Some(at) = ctx.host.location(target) else { return Err(CmdError::PlayerNotFound) };
            let loc = coords(&at, &args[1..])?;
            check_bounds(ctx, &loc)?;
            teleport_disabled(ctx, target)?;
            ctx.tl("teleporting", &loc_args(&loc));
            let job = Job::new(target, target, Target::Loc(loc.clone()), me).ok(target, "teleporting", loc_args(&loc));
            now(ctx.ess, ctx.host, job, false);
            Ok(())
        }
        _ => {
            if !ctx.has("essentials.tp.others") {
                return Err(CmdError::Tl("noPerm", vec![Arg::from("essentials.tp.others")]));
            }
            let target = player_arg(ctx, args, 0)?;
            let to = player_arg(ctx, args, 1)?;
            teleport_disabled(ctx, target)?;
            teleport_disabled(ctx, to)?;
            let (from, dest) = (world_of(ctx.host, target), world_of(ctx.host, to));
            world_denied(ctx.ess, ctx.host, me, &from, &dest)?;
            let (a, b) = (display(ctx.ess, ctx.host, user), display(ctx.ess, ctx.host, to));
            ctx.tl_to(Sender::Player(target), "teleportAtoB", &[Arg::Plain(a), Arg::Plain(b)]);
            let at = ctx.host.location(to).unwrap_or_else(|| main_spawn(ctx.host));
            let job = Job::new(target, target, Target::Player(to), me).ok(target, "teleporting", loc_args(&at));
            now(ctx.ess, ctx.host, job, false);
            Ok(())
        }
    }
}

fn tp_console(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let target = ctx.online_player(&args[0])?;
    if args.len() == 2 {
        let to = ctx.online_player(&args[1])?;
        let (a, b) = (ctx.ess.tl("consoleName", &[]), display(ctx.ess, ctx.host, to));
        ctx.tl_to(Sender::Player(target), "teleportAtoB", &[Arg::Plain(a), Arg::Plain(b)]);
        let at = ctx.host.location(to).unwrap_or_else(|| main_spawn(ctx.host));
        let job = Job::new(target, target, Target::Player(to), ctx.sender).ok(target, "teleporting", loc_args(&at));
        now(ctx.ess, ctx.host, job, false);
        Ok(())
    } else if args.len() > 3 {
        let Some(at) = ctx.host.location(target) else { return Err(CmdError::PlayerNotFound) };
        let loc = coords(&at, &args[1..])?;
        check_bounds(ctx, &loc)?;
        ctx.tl("teleporting", &loc_args(&loc));
        let job = Job::new(target, target, Target::Loc(loc.clone()), ctx.sender).ok(target, "teleporting", loc_args(&loc));
        now(ctx.ess, ctx.host, job, false);
        Ok(())
    } else {
        Err(CmdError::Usage(None))
    }
}

fn cmd_tphere(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let player = player_arg(ctx, args, 0)?;
    teleport_disabled(ctx, player)?;
    let (from, to) = (world_of(ctx.host, user), world_of(ctx.host, player));
    if from != to {
        world_denied(ctx.ess, ctx.host, ctx.sender, "", &from)?;
    }
    let at = ctx.host.location(user).unwrap_or_else(|| main_spawn(ctx.host));
    let job = Job::new(user, player, Target::Player(user), ctx.sender)
        .ok(player, "teleporting", loc_args(&at))
        .ok(user, "teleporting", loc_args(&at));
    teleport(ctx.ess, ctx.host, job, Some(user));
    Ok(())
}

fn cmd_tpo(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    match args.len() {
        0 => Err(CmdError::Usage(None)),
        1 => {
            let player = player_arg(ctx, args, 0)?;
            let (from, to) = (world_of(ctx.host, user), world_of(ctx.host, player));
            world_denied(ctx.ess, ctx.host, ctx.sender, &from, &to)?;
            let at = ctx.host.location(player).unwrap_or_else(|| main_spawn(ctx.host));
            let job = Job::new(user, user, Target::Player(player), ctx.sender).ok(user, "teleporting", loc_args(&at));
            now(ctx.ess, ctx.host, job, false);
            Ok(())
        }
        _ => {
            if !ctx.has("essentials.tp.others") {
                return Err(CmdError::Tl("noPerm", vec![Arg::from("essentials.tp.others")]));
            }
            let target = player_arg(ctx, args, 0)?;
            let to = player_arg(ctx, args, 1)?;
            let (from, dest) = (world_of(ctx.host, target), world_of(ctx.host, to));
            world_denied(ctx.ess, ctx.host, ctx.sender, &from, &dest)?;
            let at = ctx.host.location(to).unwrap_or_else(|| main_spawn(ctx.host));
            let (a, b) = (display(ctx.ess, ctx.host, user), display(ctx.ess, ctx.host, to));
            let job = Job::new(target, target, Target::Player(to), ctx.sender)
                .ok(target, "teleporting", loc_args(&at))
                .ok(target, "teleportAtoB", vec![Arg::Plain(a), Arg::Plain(b)]);
            now(ctx.ess, ctx.host, job, false);
            Ok(())
        }
    }
}

fn cmd_tpohere(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let player = player_arg(ctx, args, 0)?;
    let (from, to) = (world_of(ctx.host, user), world_of(ctx.host, player));
    if from != to {
        world_denied(ctx.ess, ctx.host, ctx.sender, "", &from)?;
    }
    let at = ctx.host.location(user).unwrap_or_else(|| main_spawn(ctx.host));
    let job = Job::new(player, player, Target::Player(user), ctx.sender).ok(player, "teleporting", loc_args(&at));
    now(ctx.ess, ctx.host, job, false);
    Ok(())
}

fn cmd_tppos(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (target, rest) = match ctx.sender {
        Sender::Player(u) => {
            if args.len() < 3 {
                return Err(CmdError::Usage(None));
            }
            (u, args)
        }
        Sender::Console => {
            if args.len() < 4 {
                return Err(CmdError::Usage(None));
            }
            (ctx.online_player(&args[0])?, &args[1..])
        }
    };
    let Some(at) = ctx.host.location(target) else { return Err(CmdError::PlayerNotFound) };
    let mut loc = coords(&at, rest)?;
    let check = ctx.is_player();
    let set_world =|ctx: &mut Ctx, loc: &mut Loc, name: &str| -> CmdResult {
        let Some(w) = find_world(ctx.host, name) else { return Err(CmdError::Tl("invalidWorld", vec![])) };
        if check {
            world_denied(ctx.ess, ctx.host, ctx.sender, &at.world, &w)?;
        }
        loc.world = w;
        Ok(())
    };
    if rest.len() == 4 {
        set_world(ctx, &mut loc, &rest[3])?;
    }
    if rest.len() > 4 {
        let f = |s: &str| s.parse::<f32>().map_err(|_| CmdError::Msg(format!("For input string: \"{s}\"")));
        loc.yaw = (f(&rest[3])? + 360.0) % 360.0;
        loc.pitch = f(&rest[4])?;
    }
    if rest.len() > 5 {
        set_world(ctx, &mut loc, &rest[5])?;
    }
    check_bounds(ctx, &loc)?;
    ctx.tl("teleporting", &loc_args(&loc));
    if ctx.sender != Sender::Player(target) {
        ctx.tl_to(Sender::Player(target), "teleporting", &loc_args(&loc));
    }
    let err_to = if check { ctx.sender } else { Sender::Player(target) };
    teleport(ctx.ess, ctx.host, Job::new(target, target, Target::Loc(loc), err_to), Some(target));
    Ok(())
}

fn cmd_tpall(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let target = match (args.first(), ctx.sender) {
        (None, Sender::Player(u)) => u,
        (None, Sender::Console) => return Err(CmdError::Usage(None)),
        (Some(a), _) => ctx.online_player(a)?,
    };
    ctx.tl("teleportAll", &[]);
    let Some(loc) = ctx.host.location(target) else { return Ok(()) };
    for p in ctx.host.online() {
        if p == target {
            continue;
        }
        if ctx.sender == Sender::Player(target) && world_of(ctx.host, p) != loc.world {
            if world_denied(ctx.ess, ctx.host, ctx.sender, "", &loc.world).is_err() {
                continue;
            }
        }
        now(ctx.ess, ctx.host, Job::new(p, p, Target::Loc(loc.clone()), ctx.sender), false);
    }
    Ok(())
}

fn cmd_tpoffline(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let Some(first) = args.first() else { return Err(CmdError::Usage(None)) };
    let target = ctx.any_player(first)?;
    let Some(logout) = user_loc(ctx.ess, target, "logoutlocation") else {
        let name = display(ctx.ess, ctx.host, target);
        ctx.tl("teleportOfflineUnknown", &[Arg::Plain(name)]);
        return Ok(());
    };
    let from = world_of(ctx.host, user);
    world_denied(ctx.ess, ctx.host, ctx.sender, &from, &logout.world)?;
    ctx.tl("teleporting", &loc_args(&logout));
    now(ctx.ess, ctx.host, Job::new(user, user, Target::Loc(logout), ctx.sender), false);
    Ok(())
}

fn match_toggle(arg: &str) -> Option<bool> {
    if arg.eq_ignore_ascii_case("on") || arg.starts_with("ena") || arg == "1" {
        Some(true)
    } else if arg.eq_ignore_ascii_case("off") || arg.starts_with("dis") || arg == "0" {
        Some(false)
    } else {
        None
    }
}

type ToggleFn = fn(&mut Ctx, Uuid, Option<bool>);

/// EssentialsToggleCommand.handleToggleWithArgs / toggleOtherPlayers.
fn toggle_command(ctx: &mut Ctx, args: &[String], others: &str, toggle: ToggleFn) -> CmdResult {
    let Sender::Player(user) = ctx.sender else { return toggle_others(ctx, args, toggle) };
    if args.len() == 1 {
        let t = match_toggle(&args[0]);
        if t.is_none() && ctx.has(others) {
            return toggle_others(ctx, args, toggle);
        }
        toggle(ctx, user, t);
    } else if args.len() == 2 && ctx.has(others) {
        return toggle_others(ctx, args, toggle);
    } else {
        toggle(ctx, user, None);
    }
    Ok(())
}

fn toggle_others(ctx: &mut Ctx, args: &[String], toggle: ToggleFn) -> CmdResult {
    let Some(search) = args.first().filter(|a| a.trim().len() >= 2) else { return Err(CmdError::PlayerNotFound) };
    // Bukkit's matchPlayer: an exact name, else every name starting with the search.
    let mut matched = Vec::new();
    for p in ctx.host.online() {
        let name = ctx.host.name(p).unwrap_or_default().to_lowercase();
        if name == search.to_lowercase() {
            matched = vec![p];
            break;
        }
        if name.starts_with(&search.to_lowercase()) {
            matched.push(p);
        }
    }
    let mut found = false;
    for p in matched {
        if !ctx.ess.can_see(ctx.host, ctx.sender, p) {
            continue;
        }
        found = true;
        toggle(ctx, p, args.get(1).and_then(|a| match_toggle(a)));
    }
    if found { Ok(()) } else { Err(CmdError::PlayerNotFound) }
}

fn cmd_tptoggle(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.tptoggle.others", |ctx, p, enabled| {
        let enabled = enabled.unwrap_or(!tp_enabled(ctx.ess, p));
        ctx.ess.users.user(p).set("teleportenabled", json!(enabled));
        ctx.tl_to(Sender::Player(p), if enabled { "teleportationEnabled" } else { "teleportationDisabled" }, &[]);
        if ctx.sender != Sender::Player(p) {
            let name = display(ctx.ess, ctx.host, p);
            ctx.tl(if enabled { "teleportationEnabledFor" } else { "teleportationDisabledFor" }, &[Arg::Plain(name)]);
        }
    })
}

fn cmd_tpauto(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.tpauto.others", |ctx, p, enabled| {
        let enabled = enabled.unwrap_or(!auto_tp(ctx.ess, p));
        ctx.ess.users.user(p).set("teleportauto", json!(enabled));
        ctx.tl_to(Sender::Player(p), if enabled { "autoTeleportEnabled" } else { "autoTeleportDisabled" }, &[]);
        if enabled && !tp_enabled(ctx.ess, p) {
            ctx.tl_to(Sender::Player(p), "teleportationDisabledWarning", &[]);
        }
        if ctx.sender != Sender::Player(p) {
            let name = display(ctx.ess, ctx.host, p);
            ctx.tl(if enabled { "autoTeleportEnabledFor" } else { "autoTeleportDisabledFor" }, &[Arg::Plain(name)]);
        }
    })
}

fn cmd_world(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
    let worlds = ctx.host.worlds();
    let world = if args.is_empty() {
        let Some(nether) = worlds.iter().find(|w| w.as_str() == NETHER).cloned() else { return Ok(()) };
        if at.world == nether { worlds[0].clone() } else { nether }
    } else {
        match find_world(ctx.host, &final_arg(args, 0)) {
            Some(w) => w,
            None => {
                ctx.tl("invalidWorld", &[]);
                ctx.tl("possibleWorlds", &[num(worlds.len() as i64 - 1)]);
                ctx.tl("typeWorldName", &[]);
                return Ok(());
            }
        }
    };
    if ctx.ess.config_bool("world-teleport-permissions", false) {
        let node = format!("essentials.worlds.{}", world_name(&world));
        if !ctx.has(&node) {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain(node)]));
        }
    }
    let factor = match (at.world.as_str(), world.as_str()) {
        (NETHER, w) if w != NETHER && w != "minecraft:the_end" => 8.0,
        (from, NETHER) if from != NETHER && from != "minecraft:the_end" => 1.0 / 8.0,
        _ => 1.0,
    };
    let b = at.block();
    let target = Loc::new(&world, b[0] as f64 * factor + 0.5, b[1] as f64, b[2] as f64 * factor + 0.5);
    teleport(ctx.ess, ctx.host, Job::new(user, user, Target::Loc(target), ctx.sender), Some(user));
    Ok(())
}

// ===== tpa requests =====================================================================================

fn request_timeout(ess: &Ess) -> i64 {
    ess.config_i64("tpa-accept-cancellation", 120)
}

fn fresh(ess: &Ess, now: i64, r: &Request) -> bool {
    let timeout = request_timeout(ess);
    timeout < 1 || now - r.time <= timeout * 1000
}

/// User.requestTeleport.
fn request_teleport(ess: &mut Ess, host: &mut dyn Host, target: Uuid, requester: Uuid, here: bool) {
    let name = host.name(requester).unwrap_or_default();
    let at = if here { host.location(requester) } else { host.location(target) };
    let Some(loc) = at else { return };
    let time = host.now_millis();
    let max = ess.config_i64("tpa-max-requests", 5).max(0) as usize;
    let queue = st(ess).requests.entry(target).or_default();
    queue.retain(|r| r.name != name);
    if !queue.is_empty() && queue.len() >= max {
        queue.pop();
    }
    queue.push(Request { name, uuid: requester, here, loc, time });
}

fn remove_request(ess: &mut Ess, target: Uuid, name: &str) -> Option<Request> {
    let queue = st(ess).requests.get_mut(&target)?;
    let i = queue.iter().position(|r| r.name == name)?;
    Some(queue.remove(i))
}

fn timed_out(ess: &mut Ess, host: &mut dyn Host, target: Uuid, r: &Request) {
    let name = display(ess, host, r.uuid);
    ess.send_tl(host, Sender::Player(target), "requestTimedOutFrom", &[Arg::Plain(name)]);
}

/// User.getOutstandingTpaRequest.
fn outstanding(ess: &mut Ess, host: &mut dyn Host, target: Uuid, name: &str, inform: bool) -> Option<Request> {
    let r = st(ess).requests.get(&target)?.iter().find(|r| r.name == name)?.clone();
    if fresh(ess, host.now_millis(), &r) {
        return Some(r);
    }
    remove_request(ess, target, name);
    if inform {
        timed_out(ess, host, target, &r);
    }
    None
}

/// User.getNextTpaRequest: newest first, dropping (and telling about) expired ones.
fn next_request(ess: &mut Ess, host: &mut dyn Host, target: Uuid, inform: bool, ignore_expirations: bool, exclude_here: bool) -> Option<Request> {
    let queue: Vec<Request> = st(ess).requests.get(&target).cloned().unwrap_or_default();
    let now = host.now_millis();
    let mut next = None;
    for r in queue.into_iter().rev() {
        if fresh(ess, now, &r) {
            if exclude_here && r.here {
                continue;
            }
            if ignore_expirations {
                return Some(r);
            }
            if next.is_none() {
                next = Some(r);
            }
        } else {
            if inform {
                timed_out(ess, host, target, &r);
            }
            remove_request(ess, target, &r.name);
        }
    }
    next
}

fn has_outstanding(ess: &mut Ess, host: &mut dyn Host, target: Uuid, name: &str, here: bool) -> bool {
    outstanding(ess, host, target, name, false).is_some_and(|r| r.here == here)
}

/// The checks /tpa and /tpahere share. Returns the target.
fn tpa_target(ctx: &mut Ctx, user: Uuid, args: &[String], here: bool) -> Result<Uuid, CmdError> {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let player = player_arg(ctx, args, 0)?;
    let (my_name, their_name) = (ctx.host.name(user).unwrap_or_default(), ctx.host.name(player).unwrap_or_default());
    if my_name.eq_ignore_ascii_case(&their_name) {
        return Err(CmdError::Usage(None));
    }
    if !has_p(ctx.ess, ctx.host, player, "essentials.tpaccept") {
        let name = display(ctx.ess, ctx.host, player);
        return Err(CmdError::Tl("teleportNoAcceptPermission", vec![Arg::Plain(name)]));
    }
    teleport_disabled(ctx, player)?;
    let (from, to) = (world_of(ctx.host, user), world_of(ctx.host, player));
    if from != to {
        world_denied(ctx.ess, ctx.host, ctx.sender, "", if here { &from } else { &to })?;
    }
    if has_outstanding(ctx.ess, ctx.host, player, &my_name, here) {
        let name = display(ctx.ess, ctx.host, player);
        return Err(CmdError::Tl("requestSentAlready", vec![Arg::Plain(name)]));
    }
    Ok(player)
}

fn send_request(ctx: &mut Ctx, from: Uuid, to: Uuid, here: bool, deny_hint: bool) {
    request_teleport(ctx.ess, ctx.host, to, from, here);
    let name = display(ctx.ess, ctx.host, from);
    let target = Sender::Player(to);
    ctx.tl_to(target, if here { "teleportHereRequest" } else { "teleportRequest" }, &[Arg::Plain(name)]);
    ctx.tl_to(target, "typeTpaccept", &[]);
    if deny_hint {
        ctx.tl_to(target, "typeTpdeny", &[]);
    }
    let timeout = request_timeout(ctx.ess);
    if timeout != 0 {
        ctx.tl_to(target, "teleportRequestTimeoutInfo", &[num(timeout)]);
    }
}

fn cmd_tpa(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let player = tpa_target(ctx, user, args, false)?;
    // TODO(social): requests from players the target ignores are dropped silently (isIgnoredPlayer).
    let (me, them) = (display(ctx.ess, ctx.host, user), display(ctx.ess, ctx.host, player));
    if auto_tp(ctx.ess, player) {
        st(ctx.ess).tp_type.insert(user, TpType::Tpa);
        ctx.tl("teleportToPlayer", &[Arg::Plain(them.clone())]);
        let job = Job::new(user, user, Target::Player(player), ctx.sender)
            .ok(player, "requestAcceptedAuto", vec![Arg::Plain(me)])
            .ok(user, "requestAcceptedFromAuto", vec![Arg::Plain(them)]);
        teleport(ctx.ess, ctx.host, job, Some(user));
        return Ok(());
    }
    send_request(ctx, user, player, false, true);
    ctx.tl("requestSent", &[Arg::Plain(them)]);
    if ctx.has("essentials.tpacancel") {
        ctx.tl("typeTpacancel", &[]);
    }
    Ok(())
}

fn cmd_tpahere(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let player = tpa_target(ctx, user, args, true)?;
    // TODO(social): ignored requesters (isIgnoredPlayer).
    send_request(ctx, user, player, true, true);
    let them = display(ctx.ess, ctx.host, player);
    ctx.tl("requestSent", &[Arg::Plain(them)]);
    ctx.tl("typeTpacancel", &[]);
    Ok(())
}

fn cmd_tpaall(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let target = match (args.first(), ctx.sender) {
        (None, Sender::Player(u)) => u,
        (None, Sender::Console) => return Err(CmdError::Usage(None)),
        (Some(a), _) => ctx.online_player(a)?,
    };
    ctx.tl("teleportAAll", &[]);
    let world = world_of(ctx.host, target);
    for p in ctx.host.online() {
        if p == target || !tp_enabled(ctx.ess, p) {
            continue;
        }
        if ctx.sender == Sender::Player(target) && world_of(ctx.host, p) != world && world_denied(ctx.ess, ctx.host, ctx.sender, "", &world).is_err() {
            continue;
        }
        send_request(ctx, target, p, true, false);
    }
    Ok(())
}

fn cmd_tpacancel(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let name = ctx.host.name(user).unwrap_or_default();
    if args.is_empty() {
        let mut count = 0;
        for p in ctx.host.online() {
            if p != user && remove_request(ctx.ess, p, &name).is_some() {
                count += 1;
            }
        }
        if count == 0 {
            return Err(CmdError::Tl("noPendingRequest", vec![]));
        }
        ctx.tl("teleportRequestAllCancelled", &[num(count)]);
    } else {
        let target = player_arg(ctx, args, 0)?;
        if remove_request(ctx.ess, target, &name).is_some() {
            let their = ctx.host.name(target).unwrap_or_default();
            ctx.tl("teleportRequestSpecificCancelled", &[Arg::Plain(their)]);
        }
    }
    Ok(())
}

fn cmd_tpaccept(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let all = args.first().is_some_and(|a| a == "*" || a.eq_ignore_ascii_case("all"));
    if next_request(ctx.ess, ctx.host, user, true, false, all).is_none() {
        return Err(CmdError::Tl("noPendingRequest", vec![]));
    }
    if all {
        let mut count = 0;
        while let Some(r) = next_request(ctx.ess, ctx.host, user, true, true, true) {
            match accept(ctx, user, Some(r.clone())) {
                Ok(()) => count += 1,
                Err(e) => show_error(ctx.ess, ctx.host, ctx.sender, &e),
            }
            remove_request(ctx.ess, user, &r.name);
        }
        ctx.tl("requestAcceptedAll", &[num(count)]);
        return Ok(());
    }
    if args.is_empty() {
        ctx.tl("requestAccepted", &[]);
        let r = next_request(ctx.ess, ctx.host, user, true, false, false);
        accept(ctx, user, r)
    } else {
        ctx.tl("requestAccepted", &[]);
        let from = player_arg(ctx, args, 0)?;
        let name = ctx.host.name(from).unwrap_or_default();
        let r = outstanding(ctx.ess, ctx.host, user, &name, true);
        accept(ctx, user, r)
    }
}

/// Commandtpaccept.handleTeleport.
fn accept(ctx: &mut Ctx, user: Uuid, request: Option<Request>) -> CmdResult {
    let none = || CmdError::Tl("noPendingRequest", vec![]);
    let Some(r) = request else { return Err(none()) };
    let requester = r.uuid;
    if !online(ctx.host, requester) {
        remove_request(ctx.ess, user, &r.name);
        return Err(none());
    }
    let (mine, theirs) = (world_of(ctx.host, user), world_of(ctx.host, requester));
    let check_world = |ctx: &mut Ctx, w: &str| mine != theirs && world_denied(ctx.ess, ctx.host, ctx.sender, "", w).is_err();
    if r.here {
        let allowed = has_p(ctx.ess, ctx.host, requester, "essentials.tpahere") || has_p(ctx.ess, ctx.host, requester, "essentials.tpaall");
        if !allowed || check_world(ctx, &mine) {
            return Err(none());
        }
    } else if !has_p(ctx.ess, ctx.host, requester, "essentials.tpa") || check_world(ctx, &theirs) {
        return Err(none());
    }
    let me = display(ctx.ess, ctx.host, user);
    ctx.tl_to(Sender::Player(requester), "requestAcceptedFrom", &[Arg::Plain(me.clone())]);
    st(ctx.ess).tp_type.insert(requester, TpType::Tpa);
    let mut job = if r.here {
        Job::new(requester, user, Target::Loc(r.loc.clone()), Sender::Player(requester)).ok(requester, "teleporting", loc_args(&r.loc))
    } else {
        ctx.tl_to(Sender::Player(requester), "teleportToPlayer", &[Arg::Plain(me)]);
        Job::new(requester, requester, Target::Player(user), Sender::Player(requester))
    };
    job.err.push(Note(Sender::Player(user), "pendingTeleportCancelled", vec![]));
    teleport(ctx.ess, ctx.host, job, Some(requester));
    remove_request(ctx.ess, user, &r.name);
    Ok(())
}

fn cmd_tpdeny(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let all = args.first().is_some_and(|a| a == "*" || a.eq_ignore_ascii_case("all"));
    if next_request(ctx.ess, ctx.host, user, false, false, false).is_none() {
        return Err(CmdError::Tl("noPendingRequest", vec![]));
    }
    let me = display(ctx.ess, ctx.host, user);
    if all {
        let mut count = 0;
        while let Some(r) = next_request(ctx.ess, ctx.host, user, false, true, false) {
            if online(ctx.host, r.uuid) {
                ctx.tl_to(Sender::Player(r.uuid), "requestDeniedFrom", &[Arg::Plain(me.clone())]);
            }
            remove_request(ctx.ess, user, &r.name);
            count += 1;
        }
        ctx.tl("requestDeniedAll", &[num(count)]);
        return Ok(());
    }
    let request = if args.is_empty() {
        next_request(ctx.ess, ctx.host, user, false, true, false)
    } else {
        let from = player_arg(ctx, args, 0)?;
        let name = ctx.host.name(from).unwrap_or_default();
        outstanding(ctx.ess, ctx.host, user, &name, false)
    };
    let Some(r) = request.filter(|r| online(ctx.host, r.uuid)) else { return Err(CmdError::Tl("noPendingRequest", vec![])) };
    ctx.tl("requestDenied", &[]);
    ctx.tl_to(Sender::Player(r.uuid), "requestDeniedFrom", &[Arg::Plain(me)]);
    remove_request(ctx.ess, user, &r.name);
    Ok(())
}

// ===== Warps (Warps.java) ===============================================================================

/// Every warp file with a name and a location: (name, file, contents).
fn warp_files(ess: &Ess) -> Vec<(String, String, Value)> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(ess.dir.join("warps")) else { return out };
    for e in dir.flatten() {
        let file = e.file_name().to_string_lossy().to_string();
        if !file.ends_with(".yml") || !e.path().is_file() {
            continue;
        }
        let conf = ess.read_yaml(&format!("warps/{file}"));
        if let (Some(name), true) = (conf.get("name").and_then(Value::as_str), conf.get("world").is_some()) {
            out.push((name.to_string(), file, conf.clone()));
        }
    }
    out
}

fn warp_list(ess: &Ess) -> Vec<String> {
    let mut names: Vec<String> = warp_files(ess).into_iter().map(|(n, _, _)| n).collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

fn warp_entry(ess: &Ess, name: &str) -> Option<(String, String, Value)> {
    warp_files(ess).into_iter().find(|(n, _, _)| n.eq_ignore_ascii_case(name))
}

fn warp_loc(ess: &Ess, name: &str) -> Result<Loc, CmdError> {
    warp_entry(ess, name).and_then(|(_, _, conf)| Loc::from_yaml(&conf)).ok_or(CmdError::Tl("warpNotExist", vec![]))
}

fn set_warp(ess: &Ess, owner: Option<Uuid>, name: &str, loc: &Loc) -> CmdResult {
    let file = match warp_entry(ess, name) {
        Some((_, file, _)) => file,
        None => {
            let file = format!("{}.yml", sanitize_file_name(name));
            if ess.dir.join("warps").join(&file).exists() {
                return Err(CmdError::Tl("similarWarpExist", vec![]));
            }
            file
        }
    };
    let mut conf = loc.to_yaml();
    conf["name"] = json!(name);
    if let Some(o) = owner {
        conf["lastowner"] = json!(uuid_string(o));
    }
    ess.write_yaml(&format!("warps/{file}"), &conf);
    Ok(())
}

fn cmd_warp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(user) = ctx.sender else {
        if args.len() < 2 || is_int(&args[0]) {
            return warp_list_cmd(ctx, args, None);
        }
        let other = ctx.online_player(&args[1])?;
        warp_to(ctx, other, other, &args[0], ctx.sender);
        return Ok(());
    };
    if args.is_empty() || args[0].chars().all(|c| c.is_ascii_digit()) {
        if !ctx.has("essentials.warp.list") {
            return Err(CmdError::Tl("warpListPermission", vec![]));
        }
        return warp_list_cmd(ctx, args, Some(user));
    }
    let mut target = user;
    if args.len() == 2 && (ctx.has("essentials.warp.otherplayers") || ctx.has("essentials.warp.others")) {
        target = player_arg(ctx, args, 1)?;
    }
    if ctx.ess.config_bool("per-warp-permission", false) && !ctx.has(&format!("essentials.warps.{}", args[0])) {
        return Err(CmdError::Tl("warpUsePermission", vec![]));
    }
    warp_to(ctx, user, target, &args[0], Sender::Player(target));
    Ok(())
}

/// AsyncTeleport.warp: `owner` sends `user` to the warp.
fn warp_to(ctx: &mut Ctx, owner: Uuid, user: Uuid, name: &str, err_to: Sender) {
    let loc = match warp_loc(ctx.ess, name) {
        Ok(l) => l,
        Err(e) => return show_error(ctx.ess, ctx.host, err_to, &e),
    };
    let mut args = vec![Arg::from(name)];
    args.extend(loc_args(&loc));
    let mut job = Job::new(owner, user, Target::Loc(loc), err_to).ok(user, "warpingTo", args.clone());
    if owner != user {
        job = job.ok(owner, "warpingTo", args);
    }
    teleport(ctx.ess, ctx.host, job, Some(owner));
}

fn warp_list_cmd(ctx: &mut Ctx, args: &[String], user: Option<Uuid>) -> CmdResult {
    let mut warps = warp_list(ctx.ess);
    if let Some(u) = user {
        if ctx.ess.config_bool("per-warp-permission", false) {
            warps.retain(|w| has_p(ctx.ess, ctx.host, u, &format!("essentials.warps.{w}")));
        }
    }
    if warps.is_empty() {
        return Err(CmdError::Tl("noWarpsDefined", vec![]));
    }
    let max_pages = warps.len().div_ceil(WARPS_PER_PAGE);
    let page = args.first().and_then(|a| a.parse::<usize>().ok()).unwrap_or(1).clamp(1, max_pages);
    let start = (page - 1) * WARPS_PER_PAGE;
    let list = warps[start..(start + WARPS_PER_PAGE).min(warps.len())].join(", ");
    if warps.len() > WARPS_PER_PAGE {
        ctx.tl("warpsCount", &[num(warps.len() as i64), num(page as i64), num(max_pages as i64)]);
        ctx.tl("warpList", &[Arg::Plain(list)]);
    } else {
        ctx.tl("warps", &[Arg::Plain(list)]);
    }
    Ok(())
}

fn cmd_warpinfo(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Some(name) = args.first() else { return Err(CmdError::Usage(None)) };
    let loc = warp_loc(ctx.ess, name)?;
    ctx.tl("warpInfo", &[Arg::from(name)]);
    ctx.tl("whoisLocation", &loc_args(&loc));
    Ok(())
}

fn cmd_setwarp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let Some(name) = args.first() else { return Err(CmdError::Usage(None)) };
    if is_int(name) || name.is_empty() || is_reserved_file_name(name) {
        return Err(CmdError::Tl("invalidWarpName", vec![]));
    }
    let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
    if warp_loc(ctx.ess, name).is_ok() && !ctx.has(&format!("essentials.warp.overwrite.{}", safe_string(name))) {
        return Err(CmdError::Tl("warpOverwrite", vec![]));
    }
    set_warp(ctx.ess, Some(user), name, &at)?;
    ctx.tl("warpSet", &[Arg::from(name)]);
    Ok(())
}

fn cmd_delwarp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Some(name) = args.first() else { return Err(CmdError::Usage(None)) };
    let Some((_, file, _)) = warp_entry(ctx.ess, name) else { return Err(CmdError::Tl("warpNotExist", vec![])) };
    if std::fs::remove_file(ctx.ess.dir.join("warps").join(file)).is_err() {
        return Err(CmdError::Tl("warpDeleteError", vec![]));
    }
    ctx.tl("deleteWarp", &[Arg::from(name)]);
    Ok(())
}

// ===== Spawn module (EssentialsSpawn) ======================================================================

/// SpawnStorage.getSpawn: the group's spawn, else `default`, else the world spawn.
fn get_spawn(ess: &Ess, host: &mut dyn Host, group: Option<&str>) -> Loc {
    let Some(group) = group else { return main_spawn(host) };
    let spawns = ess.read_yaml("spawn.yml");
    let group = group.to_lowercase();
    let at = |g: &str| spawns.get("spawns").and_then(|s| s.get(g)).and_then(Loc::from_yaml);
    at(&group).or_else(|| at("default")).unwrap_or_else(|| main_spawn(host))
}

fn group_spawn(ess: &mut Ess, host: &mut dyn Host, p: Uuid) -> Loc {
    let group = ess.group(host, p);
    get_spawn(ess, host, Some(&group))
}

fn cmd_setspawn(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    let group = if args.is_empty() { "default".to_string() } else { final_arg(args, 0) };
    let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
    let mut spawns = ctx.ess.read_yaml("spawn.yml");
    if !spawns.is_object() {
        spawns = json!({});
    }
    crate::game::ess_yaml::set(&mut spawns, &format!("spawns.{}", group.to_lowercase()), at.to_yaml());
    ctx.ess.write_yaml("spawn.yml", &spawns);
    ctx.tl("spawnSet", &[Arg::Plain(group)]);
    Ok(())
}

fn cmd_spawn(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    match ctx.sender {
        Sender::Player(user) => {
            let mut target = user;
            if !args.is_empty() && ctx.has("essentials.spawn.others") {
                target = player_arg(ctx, args, 0)?;
            }
            let spawn = group_spawn(ctx.ess, ctx.host, target);
            let mut job = Job::new(user, target, Target::Loc(spawn.clone()), ctx.sender);
            if target != user {
                let me = display(ctx.ess, ctx.host, user);
                job = job.ok(target, "teleportAtoB", vec![Arg::Plain(me), Arg::from("spawn")]);
            }
            job = job.ok(user, "teleporting", loc_args(&spawn));
            teleport(ctx.ess, ctx.host, job, Some(user));
        }
        Sender::Console => {
            let target = player_arg(ctx, args, 0)?;
            let spawn = group_spawn(ctx.ess, ctx.host, target);
            let console = ctx.ess.tl("consoleName", &[]);
            let mut job = Job::new(target, target, Target::Loc(spawn.clone()), ctx.sender).ok(target, "teleportAtoB", vec![Arg::Plain(console), Arg::from("spawn")]);
            job.ok.insert(0, Note(Sender::Console, "teleporting", loc_args(&spawn)));
            now(ctx.ess, ctx.host, job, false);
        }
    }
    Ok(())
}

fn spawn_on_join_groups(ess: &Ess) -> Vec<String> {
    match ess.config("spawn-on-join") {
        Some(Value::Bool(true)) => vec!["*".into()],
        Some(Value::Bool(false)) | None | Some(Value::Null) => Vec::new(),
        Some(_) => ess.config_list("spawn-on-join").into_iter().filter(|s| !s.is_empty()).collect(),
    }
}

fn listener_enabled(ess: &Ess, key: &str) -> bool {
    !ess.config_str(key).is_some_and(|p| p.eq_ignore_ascii_case("none"))
}

/// EssentialsSpawnPlayerListener.delayedJoin.
fn on_join(ess: &mut Ess, host: &mut dyn Host, p: Uuid, first: bool) {
    if !listener_enabled(ess, "spawn-join-listener-priority") {
        return;
    }
    if !first {
        let groups = spawn_on_join_groups(ess);
        if groups.is_empty() {
            return;
        }
        let name = host.name(p).unwrap_or_default();
        let mine = ess.perms.groups_of(p, &name);
        let in_group = groups.iter().any(|g| g == "*" || mine.contains(&g.to_lowercase()));
        if in_group && !has_p(ess, host, p, "essentials.spawn-on-join.exempt") {
            ess.scheduler.later(
                0,
                Box::new(move |ess, host| {
                    let spawn = group_spawn(ess, host, p);
                    host.teleport(p, &spawn);
                }),
            );
        }
        return;
    }
    let random = ess.config_str("random-spawn-location").unwrap_or_else(|| "none".into());
    let randomly = tpr_has_location(ess, &random);
    let newbies = ess.config_str("newbies.spawnpoint").unwrap_or_else(|| "default".into());
    if randomly || !newbies.eq_ignore_ascii_case("none") {
        ess.scheduler.later(
            1,
            Box::new(move |ess, host| {
                if !online(host, p) {
                    return;
                }
                let spawn = if randomly { random_location(ess, host, &random) } else { get_spawn(ess, host, Some(&newbies)) };
                let job = Job { owner: p, teleportee: p, target: Target::Loc(spawn), ok: Vec::new(), err_to: None, err: Vec::new() };
                now(ess, host, job, false);
            }),
        );
    }
    ess.scheduler.later(
        2,
        Box::new(move |ess, host| {
            if !online(host, p) {
                return;
            }
            let format = ess.config_str("newbies.announce-format").unwrap_or_else(|| "&dWelcome {DISPLAYNAME} to the server!".into());
            if !format.is_empty() {
                let name = display(ess, host, p);
                let text = format.replace("{DISPLAYNAME}", &name).replace("{PLAYER}", &name).replace("{USERNAME}", &name);
                for line in ess_text::legacy_to_mini(&text, true).split('\n') {
                    let spans = ess.render(line);
                    host.broadcast(&spans);
                }
            }
            // TODO(items): newbies.kit is given here (Kit.expandItems) once the items area exposes kits.
        }),
    );
}

/// EssentialsSpawnPlayerListener.onPlayerRespawn.
fn on_respawn(ess: &mut Ess, host: &mut dyn Host, p: Uuid) -> Option<Loc> {
    if !listener_enabled(ess, "respawn-listener-priority") {
        return None;
    }
    if ess.users.get(p).is_some_and(|u| u.bool("jailed") && u.str("jail").is_some_and(|j| !j.is_empty())) {
        return None;
    }
    if ess.config_bool("respawn-at-home", false) {
        let mut respawn = None;
        if ess.config_bool("respawn-at-home-bed", true) {
            if let Some(bed) = host.bed_spawn(p) {
                let b = bed.block();
                let anchor = [b, [b[0], b[1] - 1, b[2]]].iter().any(|pos| block_id(host, &bed.world, *pos) == "respawn_anchor");
                if !anchor || ess.config_bool("respawn-at-anchor", false) {
                    respawn = Some(bed);
                }
            }
        }
        let world = host.location(p).map_or_else(|| OVERWORLD.to_string(), |l| l.world);
        if let Some(home) = respawn.or_else(|| home_near(ess, p, &world)) {
            return Some(home);
        }
    }
    let random = ess.config_str("random-respawn-location").unwrap_or_else(|| "none".into());
    if tpr_has_location(ess, &random) {
        return Some(random_location(ess, host, &random));
    }
    Some(group_spawn(ess, host, p))
}

fn on_death(ess: &mut Ess, host: &mut dyn Host, p: Uuid, at: &Loc) {
    let back_disabled = ess.config_list("disabled-commands").iter().any(|c| c.eq_ignore_ascii_case("back"));
    if has_p(ess, host, p, "essentials.back.ondeath") && !back_disabled {
        set_last_location(ess, p, at);
        ess.send_tl(host, Sender::Player(p), "backAfterDeath", &[]);
    }
}

// ===== Random teleport (RandomTeleport.java, tpr.yml) ======================================================

fn tpr(ess: &Ess) -> Value {
    ess.read_yaml("tpr.yml")
}

fn tpr_has_location(ess: &Ess, name: &str) -> bool {
    tpr(ess).get("locations").and_then(|l| l.get(name)).is_some_and(|v| !v.is_null())
}

fn tpr_set(ess: &Ess, name: &str, key: &str, value: Value) {
    let mut conf = tpr(ess);
    if !conf.is_object() {
        conf = json!({});
    }
    let locations = conf.as_object_mut().expect("object").entry("locations").or_insert_with(|| json!({}));
    if !locations.is_object() {
        *locations = json!({});
    }
    let entry = locations.as_object_mut().expect("object").entry(name.to_string()).or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    entry[key] = value;
    ess.write_yaml("tpr.yml", &conf);
}

fn tpr_center(ess: &Ess, host: &mut dyn Host, name: &str) -> Loc {
    if let Some(c) = tpr(ess).get("locations").and_then(|l| l.get(name)).and_then(|l| l.get("center")).and_then(Loc::from_yaml) {
        return c;
    }
    let world = host.worlds().into_iter().next().unwrap_or_else(|| OVERWORLD.into());
    let y = host.highest_y(&world, 0, 0).map_or(64, |h| h + 1);
    let center = Loc::new(&world, 0.0, y as f64, 0.0);
    tpr_set(ess, name, "center", center.to_yaml());
    center
}

fn tpr_f64(ess: &Ess, name: &str, key: &str, default: f64) -> f64 {
    tpr(ess).get("locations").and_then(|l| l.get(name)).and_then(|l| l.get(key)).and_then(Value::as_f64).unwrap_or(default)
}

/// RandomTeleport.getRandomLocation: up to find-attempts tries, else the center.
fn random_location(ess: &mut Ess, host: &mut dyn Host, name: &str) -> Loc {
    let center = tpr_center(ess, host, name);
    let min = tpr_f64(ess, name, "min-range", 0.0);
    // The default max-range is half the vanilla world border.
    let max = tpr_f64(ess, name, "max-range", 29_999_984.0);
    let conf = tpr(ess);
    let attempts = conf.get("find-attempts").and_then(Value::as_i64).unwrap_or(10);
    let excluded: Vec<String> = match conf.get("excluded-biomes") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_lowercase)).collect(),
        _ => Vec::new(),
    };
    for _ in 0..attempts {
        let loc = calculate_random(ess, host, &center, min, max);
        let (min_y, _) = host.height_range(&loc.world);
        if loc.block()[1] <= min_y {
            continue;
        }
        let biome = host.biome(&loc.world, loc.block()).unwrap_or_default().to_lowercase();
        let short = biome.strip_prefix("minecraft:").unwrap_or(&biome).to_string();
        if excluded.contains(&biome) || excluded.contains(&short) {
            continue;
        }
        return loc;
    }
    center
}

fn calculate_random(ess: &mut Ess, host: &mut dyn Host, center: &Loc, min: f64, max: f64) -> Loc {
    let rect_x = random(ess, host) * (max - min) + min;
    let rect_z = random(ess, host) * (max + min) - min;
    let (ox, oz) = match (random(ess, host) * 4.0) as i32 {
        0 => (rect_x, rect_z),
        1 => (-rect_z, rect_x),
        2 => (-rect_x, -rect_z),
        _ => (rect_z, -rect_x),
    };
    let yaw = (360.0 * random(ess, host) - 180.0) as f32;
    let mut loc = Loc { world: center.world.clone(), x: center.x + ox, y: 0.0, z: center.z + oz, yaw, pitch: 0.0 };
    let (min_y, max_y) = host.height_range(&loc.world);
    let b = loc.block();
    let y = if loc.world == NETHER {
        let mut found = None;
        for y in 32..max_y {
            if block_id(host, &loc.world, [b[0], y, b[2]]) == "bedrock" {
                break;
            }
            if !unsafe_block(ess, host, &loc.world, b[0], y, b[2]) {
                found = Some(y);
                break;
            }
        }
        found
    } else {
        host.highest_y(&loc.world, b[0], b[2]).map(|h| h + 1)
    };
    loc.y = y.unwrap_or(min_y - 1) as f64;
    loc
}

fn cmd_tpr(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(user) = ctx.sender else {
        if args.len() < 2 {
            return Err(CmdError::Usage(None));
        }
        let target = player_arg(ctx, args, 1)?;
        if !tpr_has_location(ctx.ess, &args[0]) {
            return Err(CmdError::Tl("tprNotExist", vec![]));
        }
        ctx.tl_to(Sender::Player(target), "tprSuccess", &[]);
        let name = display(ctx.ess, ctx.host, target);
        ctx.tl("tprOtherUser", &[Arg::Plain(name)]);
        let loc = random_location(ctx.ess, ctx.host, &args[0]);
        now(ctx.ess, ctx.host, Job::new(target, target, Target::Loc(loc), ctx.sender).ok(target, "tprSuccessDone", vec![]), false);
        return Ok(());
    };
    let (name, target) = if args.is_empty() {
        let default = tpr(ctx.ess).get("default-location").and_then(Value::as_str).unwrap_or("{world}").to_string();
        let world = world_name(&world_of(ctx.host, user));
        (default.replace("{world}", &world), user)
    } else {
        let name = args[0].clone();
        if !tpr_has_location(ctx.ess, &name) {
            return Err(CmdError::Tl("tprNotExist", vec![]));
        }
        let per_location = tpr(ctx.ess).get("per-location-permission").and_then(Value::as_bool).unwrap_or(false);
        if per_location && !ctx.has(&format!("essentials.tpr.location.{name}")) {
            return Err(CmdError::Tl("tprNoPermission", vec![]));
        }
        let target = if args.len() > 1 && ctx.has("essentials.tpr.others") { player_arg(ctx, args, 1)? } else { user };
        (name, target)
    };
    ctx.tl_to(Sender::Player(target), "tprSuccess", &[]);
    if target != user {
        let n = display(ctx.ess, ctx.host, target);
        ctx.tl("tprOtherUser", &[Arg::Plain(n)]);
    }
    let loc = random_location(ctx.ess, ctx.host, &name);
    let job = Job::new(target, target, Target::Loc(loc), ctx.sender).ok(target, "tprSuccessDone", vec![]);
    teleport(ctx.ess, ctx.host, job, Some(user));
    Ok(())
}

fn cmd_settpr(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let user = ctx.player()?;
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    if args[1].eq_ignore_ascii_case("center") {
        let Some(at) = ctx.host.location(user) else { return Err(CmdError::Quiet) };
        tpr_set(ctx.ess, &args[0], "center", at.to_yaml());
        ctx.tl("settpr", &[]);
    } else if args.len() > 2 {
        let value = args[2].parse::<f64>().map_err(|_| CmdError::Msg(format!("For input string: \"{}\"", args[2])))?;
        if args[1].eq_ignore_ascii_case("minrange") {
            tpr_set(ctx.ess, &args[0], "min-range", json!(value));
        } else if args[1].eq_ignore_ascii_case("maxrange") {
            tpr_set(ctx.ess, &args[0], "max-range", json!(value));
        }
        ctx.tl("settprValue", &[Arg::Plain(args[1].to_lowercase()), Arg::Plain(args[2].to_lowercase())]);
    } else {
        return Err(CmdError::Usage(None));
    }
    Ok(())
}

// ===== Tab completion =======================================================================================

fn tab_players(ctx: &mut Ctx, _args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for p in ctx.host.online() {
        if ctx.ess.can_see(ctx.host, ctx.sender, p) {
            out.extend(ctx.host.name(p));
        }
    }
    out
}

fn tab_homes(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    if args.len() > 1 {
        return Vec::new();
    }
    let mut out = ctx.sender.player().map(|u| home_names(ctx.ess, u)).unwrap_or_default();
    if ctx.name == "home" && ctx.has("essentials.home.bed") {
        out.push("bed".into());
    }
    out
}

fn tab_warps(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    if args.len() > 1 { tab_players(ctx, args) } else { warp_list(ctx.ess) }
}

fn tab_requests(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    if args.len() > 1 {
        return Vec::new();
    }
    let mut out: Vec<String> = ctx.sender.player().and_then(|u| st(ctx.ess).requests.get(&u).map(|q| q.iter().map(|r| r.name.clone()).collect())).unwrap_or_default();
    out.push("*".into());
    out
}

fn tab_worlds(ctx: &mut Ctx, _args: &[String]) -> Vec<String> {
    ctx.host.worlds().iter().map(|w| world_name(w)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ess_testkit::*;

    const OW: &str = "minecraft:overworld";
    const NE: &str = "minecraft:the_nether";

    /// A player joins and the newbie handling (teleport at tick 1, announcement at tick 2) settles; inboxes drained.
    fn joined(t: &mut TestServer, name: &str) -> Uuid {
        let u = t.join(name);
        t.tick(3);
        drain(t);
        u
    }

    fn drain(t: &mut TestServer) {
        for p in t.host.online() {
            t.host.said(Sender::Player(p));
        }
        t.host.said(Sender::Console);
    }

    fn at(t: &mut TestServer, p: Uuid, world: &str, x: f64, y: f64, z: f64) {
        t.host.p(p).loc = Loc::new(world, x, y, z);
    }

    fn xyz(t: &mut TestServer, p: Uuid) -> (String, f64, f64, f64) {
        let l = t.loc(p);
        (l.world, l.x, l.y, l.z)
    }

    fn ow(x: f64, y: f64, z: f64) -> (String, f64, f64, f64) {
        (OW.to_string(), x, y, z)
    }

    /// permissions.yml giving `alice`, `bob`, `carol` the listed nodes (and nothing from the default group).
    fn perms(t: &mut TestServer, alice: &str, bob: &str) {
        t.permissions(&format!(
            "groups:\n  default:\n    default: true\n    permissions: []\nusers:\n  alice:\n    permissions: [{alice}]\n  bob:\n    permissions: [{bob}]\n"
        ));
    }

    // ===== Homes ==================================================================================

    #[test]
    fn sethome_and_home_with_a_single_home() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 10.5, 64.0, 10.5);
        assert_eq!(t.cmd(a, "/sethome"), ["Home set to current location."]);
        let data = t.userdata(a);
        assert_eq!(data["homes"]["home"]["x"], 10.5);
        assert_eq!(data["homes"]["home"]["world"], OW);
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/home"), ["Teleporting to home."]);
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/createhome base"), ["Home set to current location."]);
        at(&mut t, a, OW, 3.5, 64.0, 3.5);
        assert_eq!(t.cmd(a, "/home base"), ["Teleporting to base."]);
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));
    }

    #[test]
    fn home_lists_homes_and_bed() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        t.cmd(a, "/sethome mine");
        t.cmd(a, "/sethome base");
        assert_eq!(t.cmd(a, "/home"), ["Homes: base, mine, bed"]);
        assert_eq!(t.cmd(a, "/homes"), ["Homes: base, mine, bed"]);
        // A home by its number (1-based, in list order).
        at(&mut t, a, OW, 5.5, 64.0, 5.5);
        assert_eq!(t.cmd(a, "/home 2"), ["Teleporting to 2."]);
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));
    }

    #[test]
    fn home_bed() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(a, "/home bed"), ["Error: Your bed is either unset, missing or blocked."]);
        t.host.p(a).bed = Some(Loc::new(OW, 20.5, 64.0, 20.5));
        assert_eq!(t.cmd(a, "/home bed"), ["Teleporting to bed."]);
        assert_eq!(xyz(&mut t, a), ow(20.5, 64.0, 20.5));
    }

    #[test]
    fn home_without_homes_goes_to_spawn_or_errors() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 40.5, 64.0, 40.5);
        assert_eq!(t.cmd(a, "/home"), Vec::<String>::new());
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));

        let mut t = TestServer::with_config("spawn-if-no-home: false");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(a, "/home"), ["Error: Player has not set a home."]);
    }

    #[test]
    fn home_of_another_player() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        t.op("Alice");
        t.op("Bob");
        at(&mut t, b, OW, 30.5, 64.0, 30.5);
        t.cmd(b, "/sethome base");
        assert_eq!(t.cmd(a, "/home bob:base"), ["Teleporting to base."]);
        assert_eq!(xyz(&mut t, a), ow(30.5, 64.0, 30.5));
        // Several homes of another player are listed (no bed for others).
        t.cmd(b, "/sethome other");
        assert_eq!(t.cmd(a, "/home bob:"), ["Homes: base, other, bed"]);
    }

    #[test]
    fn sethome_limits_follow_sethome_multiple() {
        let mut t = TestServer::new();
        t.permissions(
            "groups:\n  default:\n    default: true\n    permissions: [essentials.sethome, essentials.home, essentials.sethome.multiple]\n  vip:\n    permissions: [essentials.sethome, essentials.home, essentials.sethome.multiple, essentials.sethome.multiple.vip]\n  single:\n    permissions: [essentials.sethome, essentials.home]\nusers:\n  carol:\n    groups: [vip]\n  bob:\n    groups: [single]\n",
        );
        let a = joined(&mut t, "Alice");
        for name in ["a", "b", "c"] {
            assert_eq!(t.cmd(a, &format!("/sethome {name}")), ["Home set to current location."]);
        }
        assert_eq!(t.cmd(a, "/sethome d"), ["Error: You cannot set more than 3 homes."]);
        // Moving an existing home is fine at the limit.
        assert_eq!(t.cmd(a, "/sethome a"), ["Home set to current location."]);

        let c = joined(&mut t, "Carol");
        for name in ["a", "b", "c", "d", "e"] {
            assert_eq!(t.cmd(c, &format!("/sethome {name}")), ["Home set to current location."]);
        }
        assert_eq!(t.cmd(c, "/sethome f"), ["Error: You cannot set more than 5 homes."]);

        // A limit of one always names the home "home".
        let b = joined(&mut t, "Bob");
        assert_eq!(t.cmd(b, "/sethome base"), ["Home set to current location."]);
        let data = t.userdata(b);
        assert!(data["homes"]["home"].is_object());
        assert!(data["homes"].get("base").is_none());
    }

    #[test]
    fn sethome_rejects_bad_names_and_unsafe_places() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(a, "/sethome bed"), ["Error: Invalid home name!"]);
        assert_eq!(t.cmd(a, "/sethome 5"), ["Error: Invalid home name!"]);
        at(&mut t, a, OW, 0.5, 80.0, 0.5);
        assert_eq!(t.cmd(a, "/sethome sky"), ["Error: The teleport destination is unsafe."]);
    }

    #[test]
    fn sethome_for_another_player() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        t.op("Alice");
        at(&mut t, a, OW, 7.5, 64.0, 7.5);
        assert_eq!(t.cmd(a, "/sethome bob:shop"), ["Home set to current location."]);
        assert_eq!(t.userdata(b)["homes"]["shop"]["x"], 7.5);
    }

    #[test]
    fn confirm_home_overwrite() {
        let mut t = TestServer::with_config("confirm-home-overwrite: true");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(a, "/sethome base"), ["Home set to current location."]);
        at(&mut t, a, OW, 9.5, 64.0, 9.5);
        assert_eq!(
            t.cmd(a, "/sethome base"),
            ["You already have a home named base!", "To overwrite your existing home, type the command again."]
        );
        assert_eq!(t.userdata(a)["homes"]["base"]["x"], 0.5);
        assert_eq!(t.cmd(a, "/sethome base"), ["Home set to current location."]);
        assert_eq!(t.userdata(a)["homes"]["base"]["x"], 9.5);
    }

    #[test]
    fn world_home_permissions() {
        let mut t = TestServer::with_config("world-home-permissions: true");
        let a = joined(&mut t, "Alice");
        perms(&mut t, "essentials.home, essentials.sethome, essentials.sethome.multiple", "");
        at(&mut t, a, NE, 4.5, 64.0, 4.5);
        t.cmd(a, "/sethome nh");
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/home nh"), ["Error: You do not have the essentials.worlds.world_nether permission."]);
        perms(&mut t, "essentials.home, essentials.sethome, essentials.worlds.world_nether", "");
        assert_eq!(t.cmd(a, "/home nh"), ["Teleporting to nh."]);
        assert_eq!(xyz(&mut t, a), (NE.to_string(), 4.5, 64.0, 4.5));
    }

    #[test]
    fn delhome() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        t.op("Alice");
        t.cmd(a, "/sethome base");
        assert_eq!(t.cmd(a, "/delhome base"), ["Home base has been removed."]);
        assert_eq!(t.cmd(a, "/delhome base"), ["Home base doesn't exist!"]);
        assert_eq!(t.cmd(a, "/delhome bed"), ["Error: Invalid home name!"]);
        assert_eq!(
            t.cmd(a, "/delhome"),
            [
                "Command Help: /delhome",
                "Description: Removes a home.",
                "Usage(s);",
                "/delhome <name> - Deletes your home with the given name",
                "/delhome <player>:<name> - Deletes the specified player's home with the given name"
            ]
        );
        t.cmd(a, "/sethome x");
        t.cmd(a, "/sethome y");
        assert_eq!(t.cmd(a, "/rmhome *"), ["Home x has been removed.", "Home y has been removed."]);
        t.op("Bob");
        t.cmd(b, "/sethome camp");
        assert_eq!(t.console("delhome bob:camp"), ["Home camp has been removed."]);
        assert!(t.userdata(b)["homes"].get("camp").is_none());
    }

    #[test]
    fn renamehome() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        t.cmd(a, "/sethome base");
        assert_eq!(t.cmd(a, "/renamehome base camp"), ["Home base has been renamed to camp."]);
        assert!(t.userdata(a)["homes"]["camp"].is_object());
        assert_eq!(t.cmd(a, "/renamehome nope x"), ["Error: Home nope doesn't exist!"]);
        assert_eq!(t.cmd(a, "/renamehome camp bed"), ["Error: Invalid home name!"]);
    }

    // ===== Teleport engine: warmup, cooldown, safety, centering, invulnerability ======================

    fn home_player(config: &str) -> (TestServer, Uuid) {
        let mut t = TestServer::with_config(config);
        let a = joined(&mut t, "Alice");
        perms(&mut t, "essentials.home, essentials.sethome, essentials.back, essentials.back.into.*", "");
        at(&mut t, a, OW, 10.5, 64.0, 10.5);
        t.cmd(a, "/sethome");
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        t.tick(100); // past the newbie teleport's invulnerability
        drain(&mut t);
        (t, a)
    }

    #[test]
    fn teleport_delay_warmup() {
        let (mut t, a) = home_player("teleport-delay: 3");
        assert_eq!(t.cmd(a, "/home"), ["Teleportation will commence in 3 seconds. Don't move."]);
        t.tick(60);
        assert!(t.said(a).is_empty());
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));
        t.tick(20);
        assert_eq!(t.said(a), ["Teleportation commencing...", "Teleporting to home."]);
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
    }

    #[test]
    fn teleport_delay_cancelled_by_moving_or_damage() {
        let (mut t, a) = home_player("teleport-delay: 3");
        t.cmd(a, "/home");
        at(&mut t, a, OW, 5.5, 64.0, 0.5);
        t.tick(20);
        assert_eq!(t.said(a), ["Pending teleportation request cancelled."]);
        t.tick(100);
        assert_eq!(xyz(&mut t, a), ow(5.5, 64.0, 0.5));

        t.cmd(a, "/home");
        t.host.p(a).health = 15.0;
        t.tick(20);
        assert_eq!(t.said(a), ["Pending teleportation request cancelled."]);
        t.tick(100);
        assert_eq!(xyz(&mut t, a), ow(5.5, 64.0, 0.5));
    }

    #[test]
    fn teleport_timer_move_and_bypass_permissions() {
        let (mut t, a) = home_player("teleport-delay: 3");
        perms(&mut t, "essentials.home, essentials.teleport.timer.move", "");
        t.cmd(a, "/home");
        at(&mut t, a, OW, 5.5, 64.0, 0.5);
        t.tick(80);
        assert_eq!(t.said(a), ["Teleportation commencing...", "Teleporting to home."]);
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));

        perms(&mut t, "essentials.home, essentials.teleport.timer.bypass", "");
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/home"), ["Teleporting to home."]);
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
    }

    #[test]
    fn teleport_cooldown() {
        let (mut t, a) = home_player("teleport-cooldown: 10");
        assert_eq!(t.cmd(a, "/home"), ["Teleporting to home."]);
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/home"), ["Error: Time before next teleport: 10 seconds."]);
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));
        t.tick(200);
        assert_eq!(t.cmd(a, "/home"), ["Teleporting to home."]);
        assert_eq!(t.userdata(a)["timestamps"]["lastteleport"], t.host.now);
        // The bypass node skips the cooldown.
        perms(&mut t, "essentials.home, essentials.teleport.cooldown.bypass", "");
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/home"), ["Teleporting to home."]);
    }

    fn op_at_spawn(config: &str) -> (TestServer, Uuid) {
        let mut t = TestServer::with_config(config);
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        (t, a)
    }

    #[test]
    fn teleport_safety_moves_off_lava_and_out_of_the_air() {
        let (mut t, a) = op_at_spawn("");
        t.host.set_block(OW, [10, 63, 10], "minecraft:lava[level=0]");
        assert_eq!(t.cmd(a, "/tppos 10.5 64 10.5"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), ow(11.5, 64.0, 11.5));

        t.host.set_block(OW, [20, 63, 20], "minecraft:air");
        t.host.set_block(OW, [20, 62, 20], "minecraft:air");
        t.cmd(a, "/tppos 20.5 64 20.5");
        assert_eq!(xyz(&mut t, a), ow(20.5, 62.0, 20.5));

        // Creative players who may fly are left where they asked.
        t.host.p(a).mode = GameMode::Creative;
        t.host.p(a).allow_flight = true;
        t.cmd(a, "/tppos 10.5 64 10.5");
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
    }

    #[test]
    fn teleport_safety_off_refuses_and_force_disable_goes_anyway() {
        let (mut t, a) = op_at_spawn("teleport-safety: false");
        t.host.set_block(OW, [10, 63, 10], "minecraft:lava");
        assert_eq!(t.cmd(a, "/tppos 10.5 64 10.5"), ["Teleporting...", "Error: The teleport destination is unsafe."]);
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));

        let (mut t, a) = op_at_spawn("force-disable-teleport-safety: true");
        t.host.set_block(OW, [10, 63, 10], "minecraft:lava");
        t.cmd(a, "/tppos 10.2 64 10.2");
        assert_eq!(xyz(&mut t, a), ow(10.2, 64.0, 10.2));
    }

    #[test]
    fn water_is_safe_only_when_configured() {
        let (mut t, a) = op_at_spawn("");
        t.host.set_block(OW, [10, 64, 10], "minecraft:water");
        t.host.set_block(OW, [10, 65, 10], "minecraft:water");
        t.cmd(a, "/tppos 10.5 64 10.5");
        assert_ne!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));

        let (mut t, a) = op_at_spawn("is-water-safe: true");
        t.host.set_block(OW, [10, 64, 10], "minecraft:water");
        t.host.set_block(OW, [10, 65, 10], "minecraft:water");
        t.cmd(a, "/tppos 10.5 64 10.5");
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
    }

    #[test]
    fn teleport_to_center() {
        let (mut t, a) = op_at_spawn("");
        t.cmd(a, "/tppos 10.2 64.4 10.7");
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
        let (mut t, a) = op_at_spawn("teleport-to-center: false");
        t.cmd(a, "/tppos 10.2 64.4 10.7");
        assert_eq!(xyz(&mut t, a), ow(10.2, 64.4, 10.7));
    }

    #[test]
    fn teleport_invulnerability() {
        let (mut t, a) = op_at_spawn("");
        t.tick(100);
        assert!(!t.host.p(a).invulnerable);
        t.cmd(a, "/tppos 10 64 10");
        assert!(t.host.p(a).invulnerable);
        assert_eq!(t.ess.on_damage(&mut t.host, a, "player_attack", 2.0), Allow::No);
        t.tick(81);
        assert!(!t.host.p(a).invulnerable);
        assert_eq!(t.ess.on_damage(&mut t.host, a, "player_attack", 2.0), Allow::Yes);

        let (mut t, a) = op_at_spawn("teleport-invulnerability: 0");
        t.tick(100);
        t.cmd(a, "/tppos 10 64 10");
        assert!(!t.host.p(a).invulnerable);
    }

    // ===== /back =====================================================================================

    #[test]
    fn back_after_teleport() {
        let (mut t, a) = home_player("");
        let mut t2 = TestServer::new();
        let fresh = t2.join("Zed");
        t2.op("Zed");
        assert_eq!(t2.cmd(fresh, "/back"), ["Error: No valid location found."]);

        at(&mut t, a, OW, 3.5, 64.0, 3.5);
        t.cmd(a, "/home");
        assert_eq!(t.cmd(a, "/back"), ["Returning to previous location."]);
        assert_eq!(xyz(&mut t, a), ow(3.5, 64.0, 3.5));
        assert_eq!(t.cmd(a, "/return"), ["Returning to previous location."]);
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 10.5));
    }

    #[test]
    fn back_needs_back_into_world() {
        let (mut t, a) = home_player("");
        perms(&mut t, "essentials.home, essentials.back", "");
        t.cmd(a, "/home");
        assert_eq!(t.cmd(a, "/back"), ["Error: You do not have the essentials.back.into.world permission."]);
    }

    #[test]
    fn back_on_death() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice"); // the default group has essentials.back.ondeath
        let death = Loc::new(OW, 50.5, 64.0, 50.5);
        t.ess.on_death(&mut t.host, a, &death);
        assert_eq!(t.said(a), ["Use the /back command to return to your death point."]);
        assert_eq!(t.userdata(a)["lastlocation"]["x"], 50.5);
        // Without the node nothing is recorded.
        perms(&mut t, "essentials.back", "");
        t.ess.on_death(&mut t.host, a, &Loc::new(OW, 70.5, 64.0, 70.5));
        assert!(t.said(a).is_empty());
        assert_eq!(t.userdata(a)["lastlocation"]["x"], 50.5);
    }

    #[test]
    fn death_and_respawn_are_seen_through_is_dead() {
        let mut t = TestServer::with_config("respawn-at-home: true");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 12.5, 64.0, 12.5);
        t.cmd(a, "/sethome");
        perms(&mut t, "essentials.back.ondeath", "");
        at(&mut t, a, OW, 50.5, 64.0, 50.5);
        t.host.dead.insert(a);
        t.tick(1);
        assert_eq!(t.said(a), ["Use the /back command to return to your death point."]);
        assert_eq!(t.userdata(a)["lastlocation"]["x"], 50.5);
        // Vanilla respawns the player at the world spawn; Essentials then moves them home.
        t.host.dead.remove(&a);
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        t.tick(1);
        assert_eq!(xyz(&mut t, a), ow(12.5, 64.0, 12.5));
    }

    #[test]
    fn back_others_and_console() {
        let (mut t, a) = home_player("");
        let b = joined(&mut t, "Bob");
        t.op("Alice");
        t.op("Bob");
        at(&mut t, b, OW, 3.5, 64.0, 3.5);
        t.cmd(b, "/tppos 8 64 8");
        drain(&mut t);
        assert_eq!(t.cmd(a, "/back Bob"), ["Returned Bob to previous location."]);
        assert_eq!(t.said(b), ["Returning to previous location."]);
        assert_eq!(xyz(&mut t, b), ow(3.5, 64.0, 3.5));
        assert_eq!(t.console("back Bob"), ["Returned Bob to previous location."]);
        assert_eq!(xyz(&mut t, b), ow(8.5, 64.0, 8.5));
    }

    // ===== top, bottom, jump ==========================================================================

    #[test]
    fn top_and_bottom() {
        let (mut t, a) = op_at_spawn("");
        at(&mut t, a, OW, 5.5, 10.0, 5.5);
        assert_eq!(t.cmd(a, "/top"), ["Teleporting to top."]);
        assert_eq!(xyz(&mut t, a), ow(5.5, 64.0, 5.5));
        // A cave at y 10-11 is the lowest safe place.
        t.host.set_block(OW, [5, 10, 5], "minecraft:air");
        t.host.set_block(OW, [5, 11, 5], "minecraft:air");
        assert_eq!(t.cmd(a, "/bottom"), ["Teleporting to bottom."]);
        assert_eq!(xyz(&mut t, a), ow(5.5, 10.0, 5.5));
    }

    #[test]
    fn jump() {
        let (mut t, a) = op_at_spawn("");
        assert_eq!(t.cmd(a, "/jump"), ["Error: That would hurt your computer's brain."]);
        t.host.p(a).target = Some([3, 63, 7]);
        assert_eq!(t.cmd(a, "/j"), Vec::<String>::new());
        assert_eq!(xyz(&mut t, a), ow(3.5, 64.0, 7.5));
        assert_eq!(t.cmd(a, "/jump lock"), ["Flying wizard mode enabled."]);
        assert_eq!(t.cmd(a, "/jump lock"), ["Flying wizard mode disabled."]);
    }

    // ===== /tp family =================================================================================

    fn two_ops() -> (TestServer, Uuid, Uuid) {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        t.op("Alice");
        t.op("Bob");
        at(&mut t, b, OW, 30.5, 64.0, 30.5);
        (t, a, b)
    }

    #[test]
    fn tp_to_player_and_player_to_player() {
        let (mut t, a, b) = two_ops();
        let c = joined(&mut t, "Carol");
        at(&mut t, b, OW, 30.5, 64.0, 30.5);
        at(&mut t, c, OW, -20.5, 64.0, 7.5);
        assert_eq!(t.cmd(a, "/tp Bob"), ["Teleporting to Bob."]);
        assert_eq!(xyz(&mut t, a), ow(30.5, 64.0, 30.5));
        assert_eq!(t.cmd(a, "/tp Bob Carol"), Vec::<String>::new());
        assert_eq!(t.said(b), ["Alice teleported you to Carol.", "Teleporting..."]);
        assert_eq!(xyz(&mut t, b), ow(-20.5, 64.0, 7.5));
        assert_eq!(t.console("tp Carol Alice"), Vec::<String>::new());
        assert_eq!(t.said(c), ["Console teleported you to Alice.", "Teleporting..."]);
        assert_eq!(xyz(&mut t, c), ow(30.5, 64.0, 30.5));
        assert_eq!(t.cmd(a, "/tp Nobody"), ["Error: Player not found."]);
    }

    #[test]
    fn tp_positions() {
        let (mut t, a, b) = two_ops();
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/tp 100 64 -50"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), ow(100.5, 64.0, -49.5));
        assert_eq!(t.cmd(a, "/tp ~5 ~ ~"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), ow(105.5, 64.0, -49.5));
        assert_eq!(t.cmd(a, "/tp Bob 10 64 10"), ["Teleporting..."]);
        assert_eq!(t.said(b), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, b), ow(10.5, 64.0, 10.5));
        let lines = t.cmd(a, "/tp 40000000 64 0");
        assert_eq!(lines.last().map(String::as_str), Some("Value of coordinates cannot be over 30000000"));
        assert_eq!(t.console("tp Bob 20 64 20"), ["Teleporting..."]);
        assert_eq!(t.said(b), ["Teleporting..."]);
    }

    #[test]
    fn tp_permission_branches() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        let _c = joined(&mut t, "Carol");
        perms(&mut t, "essentials.tp", "");
        assert_eq!(t.cmd(a, "/tp Bob Carol"), ["Error: You do not have the essentials.tp.others permission."]);
        assert_eq!(t.cmd(a, "/tp 1 64 1"), ["Error: You do not have the essentials.tp.position permission."]);
        assert_eq!(t.cmd(a, "/tp Bob"), ["Teleporting to Bob."]);
        let _ = b;
    }

    #[test]
    fn tptoggle_blocks_tp_and_tpo_ignores_it() {
        let (mut t, a, b) = two_ops();
        assert_eq!(t.cmd(b, "/tptoggle"), ["Teleportation disabled."]);
        assert_eq!(t.cmd(a, "/tp Bob"), ["Error: Bob has teleportation disabled."]);
        assert_eq!(t.cmd(a, "/tphere Bob"), ["Error: Bob has teleportation disabled."]);
        assert_eq!(t.cmd(a, "/tpo Bob"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), ow(30.5, 64.0, 30.5));
        assert_eq!(t.cmd(b, "/tptoggle on"), ["Teleportation enabled."]);
        assert_eq!(t.cmd(a, "/tptoggle Bob"), ["Teleportation disabled for Bob."]);
        assert_eq!(t.said(b), ["Teleportation disabled."]);
        assert_eq!(t.userdata(b)["teleportenabled"], false);
    }

    #[test]
    fn tphere_and_tpohere() {
        let (mut t, a, b) = two_ops();
        assert_eq!(t.cmd(a, "/tphere Bob"), ["Teleporting..."]);
        assert_eq!(t.said(b), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, b), ow(0.5, 64.0, 0.5));
        at(&mut t, b, OW, 30.5, 64.0, 30.5);
        t.cmd(b, "/tptoggle off");
        assert_eq!(t.cmd(a, "/tpohere Bob"), Vec::<String>::new());
        assert_eq!(t.said(b), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, b), ow(0.5, 64.0, 0.5));
    }

    #[test]
    fn tpoffline() {
        let (mut t, a, b) = two_ops();
        t.quit(b);
        assert_eq!(
            t.cmd(a, "/tp Bob"),
            ["Error: The player Bob is currently offline. You are able to teleport to them using /otp."]
        );
        assert_eq!(t.cmd(a, "/otp Bob"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), ow(30.5, 64.0, 30.5));
        let d = joined(&mut t, "Dave");
        t.quit(d);
        let _ = t.ess.users.user(d).data.as_object_mut().map(|o| o.remove("logoutlocation"));
        assert_eq!(t.cmd(a, "/tpoffline Dave"), ["Unable to find the last known position of Dave."]);
    }

    #[test]
    fn tppos_world_yaw_and_pitch() {
        let (mut t, a, _) = two_ops();
        assert_eq!(t.cmd(a, "/tppos 5 64 5 world_nether"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), (NE.to_string(), 5.5, 64.0, 5.5));
        t.cmd(a, "/tppos 5 64 5 -90 10 world");
        let l = t.loc(a);
        assert_eq!((l.world.as_str(), l.yaw, l.pitch), (OW, 270.0, 10.0));
        assert_eq!(t.cmd(a, "/tppos 1 2"), t.cmd(a, "/tppos"));
    }

    #[test]
    fn tpall() {
        let (mut t, a, b) = two_ops();
        let c = joined(&mut t, "Carol");
        at(&mut t, a, OW, 12.5, 64.0, 12.5);
        assert_eq!(t.cmd(a, "/tpall"), ["Teleporting all players..."]);
        assert_eq!(xyz(&mut t, b), ow(12.5, 64.0, 12.5));
        assert_eq!(xyz(&mut t, c), ow(12.5, 64.0, 12.5));
        assert_eq!(t.console("tpall Bob"), ["Teleporting all players..."]);
        assert_eq!(t.console("tpall").len(), 4); // usage
    }

    #[test]
    fn world_teleport_permissions() {
        let mut t = TestServer::with_config("world-teleport-permissions: true");
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        perms(&mut t, "essentials.tp, essentials.tpa, essentials.world", "essentials.tpaccept");
        at(&mut t, b, NE, 5.5, 64.0, 5.5);
        assert_eq!(t.cmd(a, "/tp Bob"), ["Error: You do not have the essentials.worlds.world_nether permission."]);
        assert_eq!(t.cmd(a, "/tpa Bob"), ["Error: You do not have the essentials.worlds.world_nether permission."]);
        assert_eq!(t.cmd(a, "/world world_nether"), ["Error: You do not have the essentials.worlds.world_nether permission."]);
        perms(&mut t, "essentials.tp, essentials.worlds.world_nether", "");
        assert_eq!(t.cmd(a, "/tp Bob"), ["Teleporting to Bob."]);
        assert_eq!(t.loc(a).world, NE);
    }

    #[test]
    fn world_switches_dimensions() {
        let (mut t, a, _) = two_ops();
        at(&mut t, a, OW, 80.5, 64.0, 16.5);
        assert_eq!(t.cmd(a, "/world world_nether"), Vec::<String>::new());
        assert_eq!(xyz(&mut t, a), (NE.to_string(), 10.5, 64.0, 2.5));
        t.cmd(a, "/world");
        assert_eq!(xyz(&mut t, a), ow(80.5, 64.0, 16.5));
        t.cmd(a, "/world");
        assert_eq!(t.loc(a).world, NE);
        t.cmd(a, "/world 2");
        assert_eq!(t.loc(a).world, "minecraft:the_end");
        assert_eq!(
            t.cmd(a, "/world nope"),
            ["Invalid world.", "Possible worlds are the numbers 0 through 2.", "You can also type the name of a specific world."]
        );
    }

    // ===== tpa ========================================================================================

    #[test]
    fn tpa_and_tpaccept() {
        let (mut t, a, b) = two_ops();
        assert_eq!(t.cmd(a, "/tpa Bob"), ["Request sent to Bob.", "To cancel this request, type /tpacancel."]);
        assert_eq!(
            t.said(b),
            [
                "Alice has requested to teleport to you.",
                "To teleport, type /tpaccept.",
                "To deny this request, type /tpdeny.",
                "This request will timeout after 120 seconds."
            ]
        );
        assert_eq!(t.cmd(a, "/tpa Bob"), ["Error: You have already sent Bob a teleport request."]);
        assert_eq!(t.cmd(b, "/tpaccept"), ["Teleport request accepted."]);
        assert_eq!(t.said(a), ["Bob accepted your teleport request.", "Teleporting to Bob."]);
        assert_eq!(xyz(&mut t, a), ow(30.5, 64.0, 30.5));
        assert_eq!(t.cmd(b, "/tpaccept"), ["Error: You do not have a pending request."]);
        assert_eq!(t.cmd(a, "/tpa Alice").len(), 4); // usage: not to yourself
    }

    #[test]
    fn tpa_needs_target_accept_permission() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        let _b = joined(&mut t, "Bob");
        perms(&mut t, "essentials.tpa", "");
        assert_eq!(t.cmd(a, "/tpa Bob"), ["Error: Bob does not have permission to accept teleport requests."]);
    }

    #[test]
    fn tpahere_and_tpdeny() {
        let (mut t, a, b) = two_ops();
        at(&mut t, a, OW, -5.5, 64.0, -5.5);
        assert_eq!(t.cmd(a, "/tpahere Bob"), ["Request sent to Bob.", "To cancel this request, type /tpacancel."]);
        assert_eq!(t.said(b)[0], "Alice has requested that you teleport to them.");
        // The request remembers where Alice stood.
        at(&mut t, a, OW, 40.5, 64.0, 40.5);
        assert_eq!(t.cmd(b, "/tpyes"), ["Teleport request accepted."]);
        assert_eq!(t.said(a), ["Bob accepted your teleport request.", "Teleporting..."]);
        assert_eq!(xyz(&mut t, b), ow(-5.5, 64.0, -5.5));

        t.cmd(a, "/tpa Bob");
        drain(&mut t);
        assert_eq!(t.cmd(b, "/tpdeny"), ["Teleport request denied."]);
        assert_eq!(t.said(a), ["Bob denied your teleport request."]);
        assert_eq!(t.cmd(b, "/tpno"), ["Error: You do not have a pending request."]);
    }

    #[test]
    fn tpacancel() {
        let (mut t, a, b) = two_ops();
        let c = joined(&mut t, "Carol");
        t.op("Carol");
        assert_eq!(t.cmd(a, "/tpacancel"), ["Error: You do not have a pending request."]);
        t.cmd(a, "/tpa Bob");
        t.cmd(a, "/tpa Carol");
        assert_eq!(t.cmd(a, "/tpacancel Bob"), ["Outstanding teleport request with Bob cancelled."]);
        assert_eq!(t.cmd(b, "/tpaccept"), ["Error: You do not have a pending request."]);
        assert_eq!(t.cmd(a, "/tpacancel"), ["All outstanding teleport requests cancelled."]);
        assert_eq!(t.cmd(c, "/tpaccept"), ["Error: You do not have a pending request."]);
    }

    #[test]
    fn tpa_requests_expire() {
        let mut t = TestServer::with_config("tpa-accept-cancellation: 5");
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        t.op("Alice");
        t.op("Bob");
        t.cmd(a, "/tpa Bob");
        assert_eq!(t.said(b).last().map(String::as_str), Some("This request will timeout after 5 seconds."));
        t.tick(120);
        assert_eq!(
            t.cmd(b, "/tpaccept"),
            ["Teleport request from Alice has timed out.", "Error: You do not have a pending request."]
        );
    }

    #[test]
    fn tpa_max_requests_and_accept_or_deny_all() {
        let mut t = TestServer::with_config("tpa-max-requests: 2");
        let names = ["Alice", "Bob", "Carol", "Dave"];
        let ids: Vec<Uuid> = names.iter().map(|n| joined(&mut t, n)).collect();
        for n in names {
            t.op(n);
        }
        let d = ids[3];
        for i in 0..3 {
            t.cmd(ids[i], "/tpa Dave");
        }
        // The newest request was dropped to make room: Alice and Carol remain.
        assert_eq!(t.cmd(d, "/tpaccept Bob"), ["Teleport request accepted.", "Error: You do not have a pending request."]);
        assert_eq!(t.cmd(d, "/tpaccept *"), ["Accepted 2 pending teleport request(s)."]);
        t.cmd(ids[0], "/tpa Dave");
        t.cmd(ids[1], "/tpa Dave");
        assert_eq!(t.cmd(d, "/tpdeny all"), ["Denied 2 pending teleport request(s)."]);
        assert_eq!(t.said(ids[0]).last().map(String::as_str), Some("Dave denied your teleport request."));
    }

    #[test]
    fn tpauto_accepts_automatically() {
        let (mut t, a, b) = two_ops();
        assert_eq!(t.cmd(b, "/tpauto"), ["You are now automatically approving teleport requests."]);
        assert_eq!(t.cmd(a, "/tpa Bob"), ["Teleporting to Bob.", "Bob accepted your teleport request automatically."]);
        assert_eq!(t.said(b), ["Automatically accepted a teleport request from Alice."]);
        assert_eq!(xyz(&mut t, a), ow(30.5, 64.0, 30.5));
        assert_eq!(t.cmd(b, "/tptoggle"), ["Teleportation disabled."]);
        assert_eq!(
            t.cmd(b, "/tpauto"),
            ["You are no longer automatically approving teleport requests."]
        );
        assert_eq!(
            t.cmd(b, "/tpauto"),
            ["You are now automatically approving teleport requests.", "You must enable teleportation before other players can teleport to you."]
        );
    }

    #[test]
    fn tpaall() {
        let (mut t, a, b) = two_ops();
        let c = joined(&mut t, "Carol");
        assert_eq!(t.cmd(a, "/tpaall"), ["Teleport request sent to all players..."]);
        assert_eq!(
            t.said(c),
            ["Alice has requested that you teleport to them.", "To teleport, type /tpaccept.", "This request will timeout after 120 seconds."]
        );
        assert_eq!(t.said(b)[0], "Alice has requested that you teleport to them.");
        t.op("Carol");
        t.cmd(c, "/tpaccept");
        assert_eq!(xyz(&mut t, c), ow(0.5, 64.0, 0.5));
    }

    #[test]
    fn tpa_with_warmup_cancelled_by_requester_moving() {
        let mut t = TestServer::with_config("teleport-delay: 2");
        let a = joined(&mut t, "Alice");
        let b = joined(&mut t, "Bob");
        perms(&mut t, "essentials.tpa", "essentials.tpaccept");
        at(&mut t, b, OW, 30.5, 64.0, 30.5);
        t.cmd(a, "/tpa Bob");
        t.cmd(b, "/tpaccept");
        assert_eq!(
            t.said(a),
            ["Bob accepted your teleport request.", "Teleporting to Bob.", "Teleportation will commence in 2 seconds. Don't move."]
        );
        at(&mut t, a, OW, 6.5, 64.0, 0.5);
        t.tick(20);
        assert_eq!(t.said(a), ["Pending teleportation request cancelled."]);
        t.tick(60);
        assert_eq!(xyz(&mut t, a), ow(6.5, 64.0, 0.5));
    }

    // ===== Warps ======================================================================================

    #[test]
    fn setwarp_warp_and_warpinfo() {
        let (mut t, a, _) = two_ops();
        at(&mut t, a, OW, 10.5, 64.0, 1234.5);
        assert_eq!(t.cmd(a, "/setwarp Shop"), ["Warp Shop set."]);
        let file = t.ess.read_yaml("warps/shop.yml");
        assert_eq!(file["name"], "Shop");
        assert_eq!(file["x"], 10.5);
        assert_eq!(file["lastowner"], uuid_string(a));
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/warp shop"), ["Warping to shop."]);
        assert_eq!(xyz(&mut t, a), ow(10.5, 64.0, 1234.5));
        assert_eq!(t.cmd(a, "/warpinfo Shop"), ["Information for warp Shop:", " - Location: (world, 10, 64, 1,234)"]);
        assert_eq!(t.cmd(a, "/warp nope"), ["Error: That warp does not exist."]);
        assert_eq!(t.cmd(a, "/warpinfo nope"), ["Error: That warp does not exist."]);
    }

    #[test]
    fn warp_list_and_pages() {
        let (mut t, a, _) = two_ops();
        assert_eq!(t.cmd(a, "/warp"), ["Error: No warps defined."]);
        t.cmd(a, "/setwarp beta");
        t.cmd(a, "/setwarp Alpha");
        assert_eq!(t.cmd(a, "/warps"), ["Warps: Alpha, beta"]);
        for i in 0..23 {
            t.cmd(a, &format!("/setwarp w{i:02}"));
        }
        let page1 = t.cmd(a, "/warp");
        assert_eq!(page1[0], "There are 25 warps. Showing page 1 of 2.");
        assert!(page1[1].starts_with("Alpha, beta, w00"));
        let page2 = t.cmd(a, "/warp 2");
        assert_eq!(page2, ["There are 25 warps. Showing page 2 of 2.", "w18, w19, w20, w21, w22"]);
        assert_eq!(t.cmd(a, "/warp 9")[0], "There are 25 warps. Showing page 2 of 2.");
        assert_eq!(t.console("warp")[0], "There are 25 warps. Showing page 1 of 2.");
    }

    #[test]
    fn warp_permissions() {
        let mut t = TestServer::with_config("per-warp-permission: true");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        t.cmd(a, "/setwarp shop");
        t.cmd(a, "/setwarp mine");
        let b = joined(&mut t, "Bob");
        perms(&mut t, "", "essentials.warp, essentials.warp.list, essentials.warps.shop, essentials.setwarp");
        assert_eq!(t.cmd(b, "/warp"), ["Warps: shop"]);
        assert_eq!(t.cmd(b, "/warp mine"), ["Error: You do not have permission to use that warp."]);
        assert_eq!(t.cmd(b, "/warp shop"), ["Warping to shop."]);
        assert_eq!(t.cmd(b, "/setwarp shop"), ["Error: You cannot overwrite that warp."]);
        perms(&mut t, "", "essentials.warp");
        assert_eq!(t.cmd(b, "/warp"), ["Error: You do not have permission to list warps."]);
    }

    #[test]
    fn warp_others_and_console() {
        let (mut t, a, b) = two_ops();
        at(&mut t, a, OW, 15.5, 64.0, 15.5);
        t.cmd(a, "/setwarp hub");
        assert_eq!(t.cmd(a, "/warp hub Bob"), ["Warping to hub."]);
        assert_eq!(t.said(b), ["Warping to hub."]);
        assert_eq!(xyz(&mut t, b), ow(15.5, 64.0, 15.5));
        at(&mut t, b, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.console("warp hub Bob"), Vec::<String>::new());
        assert_eq!(t.said(b), ["Warping to hub."]);
        assert_eq!(xyz(&mut t, b), ow(15.5, 64.0, 15.5));
    }

    #[test]
    fn setwarp_names_and_delwarp() {
        let (mut t, a, _) = two_ops();
        assert_eq!(t.cmd(a, "/setwarp 12"), ["Error: Invalid warp name!"]);
        assert_eq!(t.cmd(a, "/setwarp con"), ["Error: Invalid warp name!"]);
        t.cmd(a, "/setwarp my.warp");
        assert!(t.ess.dir.join("warps/my_warp.yml").exists());
        assert_eq!(t.cmd(a, "/setwarp my-warp"), ["Warp my-warp set."]);
        assert_eq!(t.cmd(a, "/setwarp my_warp"), ["Error: A warp with a similar name already exists."]);
        assert_eq!(t.cmd(a, "/delwarp my.warp"), ["Warp my.warp has been removed."]);
        assert!(!t.ess.dir.join("warps/my_warp.yml").exists());
        assert_eq!(t.console("rmwarp nope"), ["Error: That warp does not exist."]);
    }

    // ===== Spawn module ===============================================================================

    #[test]
    fn setspawn_and_spawn() {
        let (mut t, a, b) = two_ops();
        at(&mut t, a, OW, 25.5, 64.0, -25.5);
        assert_eq!(t.cmd(a, "/setspawn"), ["Spawn location set for group default."]);
        assert_eq!(t.ess.read_yaml("spawn.yml")["spawns"]["default"]["x"], 25.5);
        at(&mut t, a, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.cmd(a, "/spawn"), ["Teleporting..."]);
        assert_eq!(xyz(&mut t, a), ow(25.5, 64.0, -25.5));
        assert_eq!(t.cmd(a, "/spawn Bob"), ["Teleporting..."]);
        assert_eq!(t.said(b), ["Alice teleported you to spawn."]);
        assert_eq!(xyz(&mut t, b), ow(25.5, 64.0, -25.5));
        at(&mut t, b, OW, 0.5, 64.0, 0.5);
        assert_eq!(t.console("spawn Bob"), ["Teleporting..."]);
        assert_eq!(t.said(b), ["Console teleported you to spawn."]);
    }

    #[test]
    fn spawn_per_group() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 60.5, 64.0, 60.5);
        assert_eq!(t.cmd(a, "/setspawn VIP"), ["Spawn location set for group VIP."]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.spawn]\n  vip:\n    permissions: [essentials.spawn]\nusers:\n  bob:\n    groups: [vip]\n");
        let b = joined(&mut t, "Bob");
        let c = joined(&mut t, "Carol");
        t.cmd(b, "/spawn");
        assert_eq!(xyz(&mut t, b), ow(60.5, 64.0, 60.5));
        at(&mut t, c, OW, 9.5, 64.0, 9.5);
        t.cmd(c, "/spawn"); // no default spawn set: the world spawn
        assert_eq!(xyz(&mut t, c), ow(0.5, 64.0, 0.5));
    }

    #[test]
    fn newbies_spawnpoint_and_announcement() {
        // EssentialsX's defaults (the test kit's baseline turns newbies off).
        let mut t = TestServer::with_config("newbies:\n  announce-format: '&dWelcome {DISPLAYNAME}&d to the server!'\n  spawnpoint: newbies\n");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 70.5, 64.0, 70.5);
        t.cmd(a, "/setspawn newbies");
        let b = t.join("Bob");
        t.tick(1);
        assert_eq!(xyz(&mut t, b), ow(70.5, 64.0, 70.5));
        t.tick(1);
        // Other areas may greet a joining player too (MOTD, mail): the announcement is the last line.
        assert_eq!(t.said(a).last().map(String::as_str), Some("Welcome Bob to the server!"));
        assert_eq!(t.said(b).last().map(String::as_str), Some("Welcome Bob to the server!"));
        // Returning players are not announced or moved.
        t.quit(b);
        at(&mut t, b, OW, 3.5, 64.0, 3.5);
        t.join("Bob");
        t.tick(3);
        assert!(!t.said(a).iter().any(|l| l.contains("to the server!")));
        assert_eq!(xyz(&mut t, b), ow(3.5, 64.0, 3.5));

        let mut t = TestServer::with_config("newbies:\n  spawnpoint: none\n  announce-format: ''");
        let a = t.join("Alice");
        at(&mut t, a, OW, 3.5, 64.0, 3.5);
        t.tick(3);
        assert!(!t.said(a).iter().any(|l| l.contains("to the server!")));
        assert_eq!(xyz(&mut t, a), ow(3.5, 64.0, 3.5));
    }

    #[test]
    fn spawn_on_join() {
        let mut t = TestServer::with_config("spawn-on-join: true");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 33.5, 64.0, 33.5);
        t.cmd(a, "/setspawn");
        t.quit(a);
        at(&mut t, a, OW, 5.5, 64.0, 5.5);
        t.join("Alice");
        t.tick(2);
        assert_eq!(xyz(&mut t, a), ow(33.5, 64.0, 33.5));

        // Only the listed groups, and never the exempt.
        let mut t = TestServer::with_config("spawn-on-join: [guests]");
        let a = joined(&mut t, "Alice");
        t.quit(a);
        at(&mut t, a, OW, 5.5, 64.0, 5.5);
        t.join("Alice");
        t.tick(2);
        assert_eq!(xyz(&mut t, a), ow(5.5, 64.0, 5.5));
        t.permissions("groups:\n  guests:\n    default: true\n    permissions: [essentials.spawn-on-join.exempt]\n");
        t.quit(a);
        t.join("Alice");
        t.tick(2);
        assert_eq!(xyz(&mut t, a), ow(5.5, 64.0, 5.5));
        t.permissions("groups:\n  guests:\n    default: true\n    permissions: []\n");
        t.quit(a);
        t.join("Alice");
        t.tick(2);
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));
    }

    #[test]
    fn respawn_at_spawn_home_or_bed() {
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 44.5, 64.0, 44.5);
        t.cmd(a, "/setspawn");
        assert_eq!(t.ess.on_respawn(&mut t.host, a).map(|l| l.x), Some(44.5));

        let mut t = TestServer::with_config("respawn-at-home: true");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 12.5, 64.0, 12.5);
        t.cmd(a, "/sethome");
        assert_eq!(t.ess.on_respawn(&mut t.host, a).map(|l| l.x), Some(12.5));
        t.host.p(a).bed = Some(Loc::new(OW, 77.5, 64.0, 77.5));
        assert_eq!(t.ess.on_respawn(&mut t.host, a).map(|l| l.x), Some(77.5));
        // An anchor is only used with respawn-at-anchor.
        t.host.set_block(OW, [77, 64, 77], "minecraft:respawn_anchor[charges=1]");
        assert_eq!(t.ess.on_respawn(&mut t.host, a).map(|l| l.x), Some(12.5));

        let mut t = TestServer::with_config("respawn-listener-priority: none");
        let a = joined(&mut t, "Alice");
        assert_eq!(t.ess.on_respawn(&mut t.host, a), None);
    }

    // ===== Random teleport ============================================================================

    #[test]
    fn settpr_and_tpr() {
        let (mut t, a, _) = two_ops();
        assert_eq!(t.cmd(a, "/settpr world center"), ["Set random teleport center."]);
        assert_eq!(t.cmd(a, "/settpr world minrange 10"), ["Set random teleport minrange to 10."]);
        assert_eq!(t.cmd(a, "/settpr world maxrange 20"), ["Set random teleport maxrange to 20."]);
        let tpr = t.ess.read_yaml("tpr.yml");
        assert_eq!(tpr["locations"]["world"]["min-range"], 10.0);
        assert_eq!(tpr["locations"]["world"]["center"]["x"], 0.5);
        for _ in 0..10 {
            at(&mut t, a, OW, 0.5, 64.0, 0.5);
            assert_eq!(t.cmd(a, "/tpr"), ["Teleporting to a random location...", "You have been teleported to a random location."]);
            let l = t.loc(a);
            let reach = (l.x - 0.5).abs().max((l.z - 0.5).abs());
            assert!((9.0..=21.5).contains(&reach), "{l:?}");
            assert_eq!(l.y, 64.0);
        }
        assert_eq!(t.cmd(a, "/tpr nowhere"), ["Error: That random teleport location does not exist."]);
        assert_eq!(t.cmd(a, "/settpr world").len(), 6); // usage
    }

    #[test]
    fn tpr_excluded_biomes_fall_back_to_the_center_and_per_location_permission() {
        let (mut t, a, b) = two_ops();
        t.cmd(a, "/settpr world center");
        t.cmd(a, "/settpr world maxrange 50");
        let mut tpr = t.ess.read_yaml("tpr.yml");
        tpr["excluded-biomes"] = serde_json::json!(["plains"]);
        tpr["per-location-permission"] = serde_json::json!(true);
        t.ess.write_yaml("tpr.yml", &tpr);
        at(&mut t, a, OW, 9.5, 64.0, 9.5);
        t.cmd(a, "/tpr world");
        assert_eq!(xyz(&mut t, a), ow(0.5, 64.0, 0.5));
        perms(&mut t, "", "essentials.tpr");
        t.host.p(b).op = 0;
        assert_eq!(t.cmd(b, "/tpr world"), ["Error: You do not have permission to use that location."]);
        assert_eq!(t.console("tpr world Bob"), ["Teleporting Bob to a random location."]);
        assert_eq!(t.said(b), ["Teleporting to a random location...", "You have been teleported to a random location."]);
    }

    #[test]
    fn random_spawn_location_for_newbies() {
        let mut t = TestServer::with_config("random-spawn-location: world");
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        t.cmd(a, "/settpr world center");
        t.cmd(a, "/settpr world minrange 100");
        t.cmd(a, "/settpr world maxrange 200");
        let b = t.join("Bob");
        t.tick(2);
        let l = t.loc(b);
        assert!((l.x - 0.5).abs().max((l.z - 0.5).abs()) >= 99.0, "{l:?}");
    }

    // ===== Usage and only-players ======================================================================

    #[test]
    fn usages_and_console_only_players() {
        let (mut t, a, _) = two_ops();
        assert_eq!(
            t.cmd(a, "/tpa"),
            [
                "Command Help: /tpa",
                "Description: Request to teleport to the specified player.",
                "Usage(s);",
                "/tpa <player> - Requests to teleport to the specified player"
            ]
        );
        assert_eq!(t.console("home"), ["Error: Only in-game players can use /home."]);
        assert_eq!(t.console("sethome x"), ["Error: Only in-game players can use /sethome."]);
    }
}
