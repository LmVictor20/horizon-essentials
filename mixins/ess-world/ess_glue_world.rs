// horizon-essentials glue for area `world` (game only): `crate::game::ess_glue_world`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here), and the
// handlers of this area's hooks (horizon-mixin.toml).
//
// Player state: health, food, fire, freeze, the game mode and statistics change inside the player's own turn
// (`Game::for_player`, as vanilla commands do). Survival flight: the allowed players are kept here and carried
// on the player's body as a zero `minecraft:luck` modifier `essentials:fly`, which the library hooks
// (hook_fly_flags.rs, hook_fly_inbox.rs) read, because they cannot see this crate. God mode and Protect's damage
// rules travel the same way (`essentials:god`, `essentials:nodmg_<damage type>`) to hook_god.rs, the one place
// every hurt of a player passes. Speeds: walking through the movement_speed attribute, flying through the
// abilities packet, sent again whenever the server sends its own (join, respawn, level or game mode change).

use super::ess_glue::GameHost;
use crate::game::ess_core::*;
use horizon_protocol::packets::play::{self, Clientbound as C};
use std::collections::{HashMap, HashSet};

/// Per-player state this glue keeps (the server thread only).
#[derive(Default)]
struct WorldGlue {
    /// Players allowed to fly outside creative/spectator.
    allowed: HashSet<Uuid>,
    /// Bukkit speeds (walk, fly) set by Essentials.
    speeds: HashMap<Uuid, (f32, f32)>,
    invulnerable: HashSet<Uuid>,
    /// (time or offset, relative)
    ptime: HashMap<Uuid, (i64, bool)>,
    pweather: HashMap<Uuid, bool>,
    /// What was last seen of each player: (entity id, dimension, game mode).
    seen: HashMap<Uuid, (i32, String, i32)>,
    /// The body modifiers applied: fly, god, nodmg_* ids.
    modifiers: HashMap<Uuid, (i32, HashSet<String>)>,
    /// Death rules per player, refreshed every second for the death hook.
    death: HashMap<Uuid, crate::game::ess_world::DeathSettings>,
    /// Inventories and experience kept through a death until the respawn.
    kept: HashMap<Uuid, Kept>,
    ticks: u64,
}

type Stack = Option<horizon_client::items::ItemStack>;

struct Kept {
    inventory: Option<(Vec<Stack>, [Stack; 4], Stack)>,
    xp: Option<(i32, f32, i32)>,
}

thread_local! {
    static GLUE: std::cell::RefCell<WorldGlue> = std::cell::RefCell::new(WorldGlue::default());
}

fn glue<T>(f: impl FnOnce(&mut WorldGlue) -> T) -> T {
    GLUE.with_borrow_mut(f)
}

const SURVIVAL: i32 = 0;
const CREATIVE: i32 = 1;
const ADVENTURE: i32 = 2;
const SPECTATOR: i32 = 3;

fn mode_id(mode: GameMode) -> i32 {
    match mode {
        GameMode::Survival => SURVIVAL,
        GameMode::Creative => CREATIVE,
        GameMode::Adventure => ADVENTURE,
        GameMode::Spectator => SPECTATOR,
    }
}

impl GameHost<'_, '_> {
    /// Runs `f` inside the player's turn (their body, inventory and stats in the game's place).
    fn in_turn<T>(&mut self, player: Uuid, f: impl FnOnce(&mut super::Game<'_>) -> T) -> Option<T> {
        let conn = self.conn(player)?;
        self.game.for_player(super::commands::Target::Guest(conn), f)
    }

    /// The abilities packet as Bukkit would send it: our flight and speeds over the game mode's.
    fn send_abilities(&mut self, player: Uuid) {
        let (allowed, (walk, fly)) = glue(|g| (g.allowed.contains(&player), g.speeds.get(&player).copied().unwrap_or((0.2, 0.1))));
        let Some(guest) = self.guest(player) else { return };
        let mut flags = guest.ability_flags();
        if allowed {
            flags |= 4;
            if guest.flying {
                flags |= 2;
            }
        }
        let conn = guest.conn;
        if let Some(hub) = self.hub() {
            hub.send_to(conn, &C::PlayerAbilities(play::PlayerAbilities { flags, flying_speed: fly / 2.0, walking_speed: walk / 2.0 }));
        }
    }

    fn console(&mut self, line: &str) -> (bool, Vec<String>) {
        self.console_command(line)
    }

    /// Adds or removes a zero `minecraft:luck` modifier the library hooks read.
    fn body_flag(&mut self, player: Uuid, id: &str, on: bool) {
        // Transient, on the body: a new body (respawn) starts without it and `sync_flags` adds it again.
        let id = format!("essentials:{id}");
        self.in_turn(player, |g| {
            if let Some(p) = g.player.as_mut() {
                let _ = p.set_modifier("minecraft:luck", &id, 0.0, "add_value", on);
            }
        });
    }

    /// Makes the body flags match `wanted`; a new body (another entity id) gets them all again.
    fn sync_flags(&mut self, player: Uuid, wanted: HashSet<String>) {
        let Some(entity) = self.guest(player).map(|g| g.entity_id) else { return };
        let (old_entity, old) = glue(|g| g.modifiers.get(&player).cloned()).unwrap_or((entity, HashSet::new()));
        let old = if old_entity == entity { old } else { HashSet::new() };
        if old == wanted {
            glue(|g| g.modifiers.insert(player, (entity, wanted)));
            return;
        }
        for id in old.difference(&wanted) {
            self.body_flag(player, id, false);
        }
        for id in wanted.difference(&old) {
            self.body_flag(player, id, true);
        }
        glue(|g| g.modifiers.insert(player, (entity, wanted)));
    }

    fn wanted_flags(&mut self, player: Uuid, god: bool, nodmg: &[&str]) -> HashSet<String> {
        let mut out: HashSet<String> = nodmg.iter().map(|d| format!("nodmg_{d}")).collect();
        if god {
            out.insert("god".into());
        }
        let creative = self.guest(player).is_some_and(|g| g.game_mode == CREATIVE || g.game_mode == SPECTATOR);
        if glue(|g| g.allowed.contains(&player)) && !creative {
            out.insert("fly".into());
        }
        out
    }

    fn time_packet(&mut self, player: Uuid) {
        let Some(conn) = self.conn(player) else { return };
        let day = self.game.world.day_time;
        let game_time = self.game.world.game_time;
        let (total, rate) = match glue(|g| g.ptime.get(&player).copied()) {
            Some((offset, true)) => (day + offset, 1.0),
            Some((fixed, false)) => (fixed, 0.0),
            None => (day, 1.0),
        };
        // The overworld clock is the first world_clock entry (`minecraft:overworld` sorts before the End's).
        let packet = C::SetTime(play::SetTime { game_time, clocks: vec![play::ClockState { clock: 0, total_ticks: total, partial_tick: 0.0, rate }] });
        if let Some(hub) = self.hub() {
            hub.send_to(conn, &packet);
        }
    }

    fn weather_packet(&mut self, player: Uuid) {
        let Some(conn) = self.conn(player) else { return };
        let raining = match glue(|g| g.pweather.get(&player).copied()) {
            Some(d) => d,
            None => self.game.world.weather.raining,
        };
        let level = if raining { 1.0 } else { 0.0 };
        if let Some(hub) = self.hub() {
            hub.send_to(conn, &C::GameEvent(play::GameEvent { event: if raining { 1 } else { 2 }, param: 0.0 }));
            hub.send_to(conn, &C::GameEvent(play::GameEvent { event: 7, param: level }));
        }
    }

    /// A vanilla command's "Killed N entities" / "Killed <name>" feedback as a count.
    fn killed(lines: &[String]) -> usize {
        for l in lines {
            if let Some(rest) = l.strip_prefix("Killed ") {
                if let Some(n) = rest.strip_suffix(" entities").and_then(|n| n.trim().parse().ok()) {
                    return n;
                }
                return 1;
            }
        }
        0
    }
}

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn world_game_mode(&mut self, player: Uuid) -> GameMode {
        match self.guest(player).map(|g| g.game_mode) {
            Some(CREATIVE) => GameMode::Creative,
            Some(ADVENTURE) => GameMode::Adventure,
            Some(SPECTATOR) => GameMode::Spectator,
            _ => GameMode::Survival,
        }
    }
    pub(super) fn world_set_game_mode(&mut self, player: Uuid, mode: GameMode) {
        use horizon_client::command::args::GameMode as M;
        let m = match mode {
            GameMode::Survival => M::Survival,
            GameMode::Creative => M::Creative,
            GameMode::Adventure => M::Adventure,
            GameMode::Spectator => M::Spectator,
        };
        self.in_turn(player, |g| g.set_game_mode(m));
        if let Some(g) = self.guest_mut(player) {
            g.game_mode = mode_id(mode);
        }
        // Bukkit keeps allowFlight across survival modes only when a plugin sets it again.
        if matches!(mode, GameMode::Creative | GameMode::Spectator) {
            glue(|g| g.allowed.insert(player));
        } else {
            glue(|g| g.allowed.remove(&player));
        }
        self.send_abilities(player);
    }
    pub(super) fn world_health(&mut self, player: Uuid) -> (f64, f64) {
        let health = self.guest(player).map_or(20.0, |g| f64::from(g.health));
        let max = self.in_turn(player, |g| g.player.as_mut().and_then(|p| p.attribute("minecraft:max_health").ok())).flatten().unwrap_or(20.0);
        (health, max)
    }
    pub(super) fn world_set_health(&mut self, player: Uuid, health: f64) {
        self.in_turn(player, |g| {
            if let Some(p) = g.player.as_mut() {
                let max = p.attribute("minecraft:max_health").unwrap_or(20.0);
                let _ = p.set_health(health.clamp(0.0, max) as f32, false);
            }
        });
        if let Some(g) = self.guest_mut(player) {
            g.health = health as f32;
        }
    }
    pub(super) fn world_food(&mut self, player: Uuid) -> (i32, f32) {
        self.guest(player).map_or((20, 5.0), |g| (g.food, g.saturation))
    }
    pub(super) fn world_set_food(&mut self, player: Uuid, food: i32, saturation: f32) {
        self.in_turn(player, |g| {
            if let Some(p) = g.player.as_mut() {
                p.health.food.food = food.clamp(0, 20);
                p.health.food.saturation = saturation.clamp(0.0, food.clamp(0, 20) as f32);
                p.health.food.exhaustion = 0.0;
            }
        });
        if let Some(g) = self.guest_mut(player) {
            g.food = food.clamp(0, 20);
            g.saturation = saturation;
        }
    }
    pub(super) fn world_set_fire_ticks(&mut self, player: Uuid, ticks: i32) {
        self.in_turn(player, |g| {
            if let Some(p) = g.player.as_mut() {
                p.fire_ticks = ticks;
            }
        });
    }
    pub(super) fn world_fire_ticks(&mut self, player: Uuid) -> i32 {
        self.in_turn(player, |g| g.player.as_ref().map_or(0, |p| p.fire_ticks)).unwrap_or(0)
    }
    pub(super) fn world_set_freeze_ticks(&mut self, player: Uuid, ticks: i32) {
        self.in_turn(player, |g| {
            if let Some(p) = g.player.as_mut() {
                p.ticks_frozen = ticks;
            }
        });
    }
    pub(super) fn world_exp(&mut self, player: Uuid) -> i64 {
        self.guest(player).map_or(0, |g| i64::from(g.xp_total))
    }
    pub(super) fn world_set_exp(&mut self, player: Uuid, total: i64) {
        let Some(name) = self.guest(player).map(|g| g.name.clone()) else { return };
        self.console(&format!("xp set {name} 0 levels"));
        self.console(&format!("xp set {name} 0 points"));
        if total > 0 {
            self.console(&format!("xp add {name} {} points", total.min(i32::MAX as i64)));
        }
    }
    pub(super) fn world_flying(&mut self, player: Uuid) -> bool {
        self.guest(player).is_some_and(|g| g.flying)
    }
    pub(super) fn world_set_allow_flight(&mut self, player: Uuid, allow: bool) {
        let creative = self.guest(player).is_some_and(|g| g.game_mode == CREATIVE || g.game_mode == SPECTATOR);
        glue(|g| if allow { g.allowed.insert(player) } else { g.allowed.remove(&player) });
        if !allow && !creative {
            if let Some(g) = self.guest_mut(player) {
                g.flying = false;
            }
        }
        if !creative {
            self.body_flag(player, "fly", allow);
            if let Some(entity) = self.guest(player).map(|g| g.entity_id) {
                glue(|g| {
                    let entry = g.modifiers.entry(player).or_insert_with(|| (entity, HashSet::new()));
                    if allow {
                        entry.1.insert("fly".into());
                    } else {
                        entry.1.remove("fly");
                    }
                });
            }
        }
        self.send_abilities(player);
    }
    pub(super) fn world_allow_flight(&mut self, player: Uuid) -> bool {
        let creative = self.guest(player).is_some_and(|g| g.game_mode == CREATIVE || g.game_mode == SPECTATOR);
        creative || glue(|g| g.allowed.contains(&player))
    }
    pub(super) fn world_set_flying(&mut self, player: Uuid, flying: bool) {
        let may = self.world_allow_flight(player);
        if let Some(g) = self.guest_mut(player) {
            g.flying = flying && may;
        }
        self.send_abilities(player);
    }
    pub(super) fn world_set_fly_speed(&mut self, player: Uuid, speed: f32) {
        glue(|g| g.speeds.entry(player).or_insert((0.2, 0.1)).1 = speed);
        self.send_abilities(player);
    }
    pub(super) fn world_set_walk_speed(&mut self, player: Uuid, speed: f32) {
        glue(|g| g.speeds.entry(player).or_insert((0.2, 0.1)).0 = speed);
        // Bukkit: the movement_speed attribute's base is half the walk speed (0.2 -> 0.1).
        self.in_turn(player, |g| {
            if let Some(p) = g.player.as_mut() {
                let key = p.attributes.key("minecraft:movement_speed");
                if let Some(handle) = p.attributes.get_instance(key).ok().flatten() {
                    let _ = p.attributes.edit(&handle, |value, observer| value.set_base_value(f64::from(speed) / 2.0, observer));
                }
            }
        });
        self.send_abilities(player);
    }
    pub(super) fn world_speeds(&mut self, player: Uuid) -> (f32, f32) {
        glue(|g| g.speeds.get(&player).copied().unwrap_or((0.2, 0.1)))
    }
    pub(super) fn world_set_invulnerable(&mut self, player: Uuid, invulnerable: bool) {
        glue(|g| if invulnerable { g.invulnerable.insert(player) } else { g.invulnerable.remove(&player) });
    }
    pub(super) fn world_set_player_time(&mut self, player: Uuid, time: Option<(i64, bool)>) {
        glue(|g| match time {
            Some(t) => g.ptime.insert(player, t),
            None => g.ptime.remove(&player),
        });
        self.time_packet(player);
    }
    pub(super) fn world_set_player_weather(&mut self, player: Uuid, downfall: Option<bool>) {
        glue(|g| match downfall {
            Some(d) => g.pweather.insert(player, d),
            None => g.pweather.remove(&player),
        });
        self.weather_packet(player);
    }
    pub(super) fn world_entity_types(&mut self) -> Vec<String> {
        Vec::new()
    }
    pub(super) fn world_world_time(&mut self, world: &str) -> i64 {
        self.game.world.day_time
    }
    pub(super) fn world_set_world_time(&mut self, world: &str, time: i64) {
        // As `time set` does: the overworld clock is the one every level shows.
        self.game.world.day_time = time.max(0);
        self.game.ui.clock.partial = 0.0;
        let players: Vec<Uuid> = glue(|g| g.ptime.keys().copied().collect());
        for p in players {
            self.time_packet(p);
        }
    }
    pub(super) fn world_weather(&mut self, world: &str) -> (bool, bool) {
        let w = &self.game.world.weather;
        (w.raining, w.thundering)
    }
    pub(super) fn world_set_weather(&mut self, world: &str, rain: bool, thunder: bool, duration: Option<i32>) {
        // ServerLevel.setWeatherParameters, as `weather` does.
        let w = &mut self.game.world.weather;
        let d = duration.unwrap_or(if rain { 12_000 } else { 60_000 });
        w.raining = rain;
        w.thundering = thunder;
        if rain || thunder {
            w.clear_time = 0;
            w.rain_time = d;
            w.thunder_time = d;
        } else {
            w.clear_time = d;
            w.rain_time = 0;
            w.thunder_time = 0;
        }
    }
    pub(super) fn world_spawn_entity(&mut self, kind: &str, at: &Loc, snbt: Option<&str>) -> bool {
        let nbt = snbt.map(|s| format!(" {s}")).unwrap_or_default();
        let (ok, _) = self.console(&format!("execute in {} run summon {kind} {} {} {}{nbt}", at.world, at.x, at.y, at.z));
        ok
    }
    pub(super) fn world_strike_lightning(&mut self, at: &Loc, effect_only: bool) {
        if super::Dim::from_id(&at.world) == self.game.dim {
            self.game.spawn_bolt([at.x, at.y, at.z], effect_only);
        } else {
            self.console(&format!("execute in {} run summon minecraft:lightning_bolt {} {} {}", at.world, at.x, at.y, at.z));
        }
    }
    pub(super) fn world_set_statistic(&mut self, player: Uuid, stat: &str, value: i64) {
        // Only resets exist in Horizon's stats (what /rest needs).
        if value == 0 {
            if let Some((ty, key)) = stat.split_once('/') {
                let ty = ty.trim_start_matches("minecraft:").to_string();
                let key = key.to_string();
                self.in_turn(player, |g| g.stats.reset(&ty, &key));
            }
        }
    }
    pub(super) fn world_remove_entities(&mut self, world: &str, center: Option<&Loc>, radius: Option<f64>, kinds: &[String]) -> usize {
        let tag = kinds.iter().find_map(|k| k.strip_prefix("#tag:")).map(|t| format!(",tag={t}")).unwrap_or_default();
        let place = match (center, radius) {
            (Some(c), Some(_)) => format!(" positioned {} {} {}", c.x, c.y, c.z),
            _ => String::new(),
        };
        let range = radius.map(|r| format!(",distance=..{r}")).unwrap_or_default();
        let mut removed = 0;
        for kind in kinds.iter().filter(|k| !k.starts_with('#')) {
            let (_, lines) = self.console(&format!("execute in {world}{place} run kill @e[type={kind}{range}{tag}]"));
            removed += Self::killed(&lines);
        }
        removed
    }
}

// ----- This area's tick (hook_world_tick.rs) ------------------------------------------------------

/// Once per server tick, after Essentials' own: abilities sent again after the server's own, per-player time
/// and weather kept, body flags and death rules mirrored from Essentials every second, kept inventories back
/// after a respawn.
pub(super) fn on_tick(game: &mut super::Game<'_>) {
    if game.native_window() {
        return;
    }
    let second = glue(|g| {
        g.ticks += 1;
        g.ticks % 20 == 0
    });
    let mut host = GameHost { game: &mut *game };
    let online = host.online();
    for p in &online {
        let Some((entity, dim, mode)) = host.guest(*p).map(|g| (g.entity_id, g.dimension.clone(), g.game_mode)) else { continue };
        let now = (entity, dim, mode);
        let before = glue(|g| g.seen.insert(*p, now.clone()));
        if before.as_ref() != Some(&now) {
            let custom = glue(|g| g.allowed.contains(p) || g.speeds.contains_key(p));
            if custom {
                host.send_abilities(*p);
            }
        }
        restore_kept(&mut host, *p);
        if second {
            if glue(|g| g.ptime.contains_key(p)) {
                host.time_packet(*p);
            }
            if glue(|g| g.pweather.contains_key(p)) {
                host.weather_packet(*p);
            }
        }
    }
    glue(|g| g.seen.retain(|u, _| online.contains(u)));
    if !second {
        return;
    }
    let rules = super::ess_glue::with_ess(game, |ess, host| {
        let mut out = Vec::new();
        for p in host.online() {
            let (god, nodmg) = crate::game::ess_world::blocked_damage(ess, host, p);
            let death = crate::game::ess_world::death_settings(ess, host, p);
            out.push((p, god, nodmg, death));
        }
        out
    });
    let mut host = GameHost { game: &mut *game };
    for (p, god, nodmg, death) in rules.unwrap_or_default() {
        let wanted = host.wanted_flags(p, god, &nodmg);
        host.sync_flags(p, wanted);
        glue(|g| g.death.insert(p, death));
    }
}

fn restore_kept(host: &mut GameHost, player: Uuid) {
    let alive = host.guest(player).is_some_and(|g| !g.dead && g.health > 0.0);
    if !alive || !glue(|g| g.kept.contains_key(&player)) {
        return;
    }
    let Some(kept) = glue(|g| g.kept.remove(&player)) else { return };
    if let Some(g) = host.guest_mut(player) {
        if let Some((inv, armor, offhand)) = kept.inventory {
            for (slot, item) in inv.into_iter().enumerate() {
                if item.is_some() && slot < g.inventory.len() {
                    g.inventory[slot] = item;
                }
            }
            for (slot, item) in armor.into_iter().enumerate() {
                if item.is_some() {
                    g.armor[slot] = item;
                }
            }
            if offhand.is_some() {
                g.offhand = offhand;
            }
        }
        if let Some((level, progress, total)) = kept.xp {
            g.xp_level = level;
            g.xp_progress = progress;
            g.xp_total = total;
        }
    }
}

// ----- Death (hook_world_death.rs) ----------------------------------------------------------------

/// Before a guest's death is processed: keepinv / keepxp (EssentialsEntityListener) take the inventory and the
/// experience out of the drops until the respawn; cursed items follow vanishing/binding-items-policy.
pub(super) fn on_guest_die(host: &mut super::lan::LanHost, conn: u32) {
    let Some(g) = host.hub.guests.iter_mut().find(|g| g.conn == conn) else { return };
    if g.health > 0.0 || g.dead {
        return;
    }
    let Some(rules) = glue(|w| w.death.get(&g.uuid).cloned()) else { return };
    let mut kept = Kept { inventory: None, xp: None };
    if rules.keep_inventory {
        let curse = |s: &Stack| -> Option<String> {
            let s = s.as_ref()?;
            if rules.vanishing != "keep" && s.enchantment_level("minecraft:vanishing_curse") > 0 {
                Some(rules.vanishing.clone())
            } else if rules.binding != "keep" && s.enchantment_level("minecraft:binding_curse") > 0 {
                Some(rules.binding.clone())
            } else {
                None
            }
        };
        // Items to drop stay in place (vanilla drops them); deleted ones go; the rest is kept.
        let mut take = |slot: &mut Stack| -> Stack {
            match curse(slot).as_deref() {
                Some("drop") => None,
                Some(_) => {
                    *slot = None;
                    None
                }
                None => slot.take(),
            }
        };
        let inv: Vec<Stack> = g.inventory.iter_mut().map(&mut take).collect();
        let armor = [take(&mut g.armor[0]), take(&mut g.armor[1]), take(&mut g.armor[2]), take(&mut g.armor[3])];
        let offhand = take(&mut g.offhand);
        kept.inventory = Some((inv, armor, offhand));
    }
    if rules.keep_level {
        kept.xp = Some((g.xp_level, g.xp_progress, g.xp_total));
        g.xp_level = 0;
        g.xp_progress = 0.0;
        g.xp_total = 0;
    }
    if kept.inventory.is_some() || kept.xp.is_some() {
        let uuid = g.uuid;
        glue(|w| w.kept.insert(uuid, kept));
    }
}

// ----- Network (Discord REST, GeoIP download, XMPP) ------------------------------------------------
// Requests run on their own threads (ureq is blocking); answers wait in NET until the next poll, so the server
// thread never blocks on the network.

use std::sync::Mutex;
use std::sync::mpsc;

struct Net {
    http_done: Vec<(u64, u16, String)>,
    tcp_events: Vec<(u64, Vec<u8>)>,
    tcp_writers: std::collections::HashMap<u64, mpsc::Sender<Vec<u8>>>,
}

static NET: Mutex<Option<Net>> = Mutex::new(None);

fn net<T>(f: impl FnOnce(&mut Net) -> T) -> T {
    let mut guard = NET.lock().unwrap_or_else(|e| e.into_inner());
    let n = guard.get_or_insert_with(|| Net { http_done: Vec::new(), tcp_events: Vec::new(), tcp_writers: Default::default() });
    f(n)
}

impl GameHost<'_, '_> {
    pub(super) fn world_http_request(&mut self, id: u64, method: &str, url: &str, headers: &[(String, String)], body: Option<&str>) {
        let (method, url, headers, body) = (method.to_string(), url.to_string(), headers.to_vec(), body.map(str::to_string));
        std::thread::spawn(move || {
            let mut req = ureq::request(&method, &url).timeout(std::time::Duration::from_secs(20));
            for (k, v) in &headers {
                req = req.set(k, v);
            }
            let result = match &body {
                Some(b) => req.send_string(b),
                None => req.call(),
            };
            let answer = match result {
                Ok(resp) => {
                    let status = resp.status();
                    (status, resp.into_string().unwrap_or_default())
                }
                Err(ureq::Error::Status(code, resp)) => (code, resp.into_string().unwrap_or_default()),
                Err(e) => (0, e.to_string()),
            };
            net(|n| n.http_done.push((id, answer.0, answer.1)));
        });
    }
    pub(super) fn world_http_poll(&mut self) -> Vec<(u64, u16, String)> {
        net(|n| std::mem::take(&mut n.http_done))
    }
    pub(super) fn world_tcp_open(&mut self, id: u64, host: &str, port: u16, tls: bool) {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        net(|n| n.tcp_writers.insert(id, tx));
        let host = host.to_string();
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            // TLS streams are not reachable through the crates Horizon depends on (ureq keeps rustls private).
            let stream = if tls { None } else { std::net::TcpStream::connect((host.as_str(), port)).ok() };
            let Some(mut stream) = stream else {
                net(|n| n.tcp_events.push((id, Vec::new())));
                return;
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
            let mut buf = [0u8; 8192];
            loop {
                loop {
                    match rx.try_recv() {
                        Ok(data) if data.is_empty() => {
                            net(|n| n.tcp_events.push((id, Vec::new())));
                            return;
                        }
                        Ok(data) => {
                            if stream.write_all(&data).is_err() {
                                net(|n| n.tcp_events.push((id, Vec::new())));
                                return;
                            }
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => return,
                    }
                }
                match stream.read(&mut buf) {
                    Ok(0) => {
                        net(|n| n.tcp_events.push((id, Vec::new())));
                        return;
                    }
                    Ok(k) => net(|n| n.tcp_events.push((id, buf[..k].to_vec()))),
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(_) => {
                        net(|n| n.tcp_events.push((id, Vec::new())));
                        return;
                    }
                }
            }
        });
    }
    pub(super) fn world_tcp_send(&mut self, id: u64, data: &[u8]) {
        if !data.is_empty() {
            net(|n| n.tcp_writers.get(&id).map(|w| w.send(data.to_vec())));
        }
    }
    pub(super) fn world_tcp_close(&mut self, id: u64) {
        net(|n| n.tcp_writers.remove(&id).map(|w| w.send(Vec::new())));
    }
    pub(super) fn world_tcp_poll(&mut self) -> Vec<(u64, Vec<u8>)> {
        net(|n| std::mem::take(&mut n.tcp_events))
    }
}
