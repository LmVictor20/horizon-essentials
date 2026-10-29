// horizon-essentials glue (game only): `crate::game::ess_glue`. Owns the running Essentials and implements
// `Host` over Horizon's Game and the server hub. Each Host method an area needs is delegated to that area's
// glue module (`ess_glue_<area>`, in mixins/ess-<area>/), so areas never edit this file.
//
// Commands: the native_command hook claims Essentials' labels and queues them; native_tick runs them (and
// everything else) with every player in the hub, no guest swapped into Game.

use crate::game::ess_core::{self, Ess, GameMode, Host, Item, Loc, Sender, ServerStats, Span, Station, Uuid};
use crate::game::ess_text::Style;
use horizon_protocol::packets::play::{self, Clientbound as C};
use serde_json::{Value, json};
use std::cell::RefCell;

thread_local! {
    /// The running Essentials (None until the first server tick, and while a call is in progress).
    static ESS: RefCell<Option<Ess>> = const { RefCell::new(None) };
    /// Command lines claimed but not yet run: (sender, line).
    static QUEUE: RefCell<Vec<(Sender, String)>> = const { RefCell::new(Vec::new()) };
    /// Players seen online last tick (joins and quits are their difference).
    static ONLINE: RefCell<Vec<Uuid>> = const { RefCell::new(Vec::new()) };
    /// Essentials started once (it is None only while a call holds it, never started twice).
    static STARTED: RefCell<bool> = const { RefCell::new(false) };
    /// True while the glue itself runs a vanilla command: the command hook must not claim it.
    static RUNNING_VANILLA: RefCell<bool> = const { RefCell::new(false) };
}

/// Essentials' view of the server: a borrowed Game (the dedicated server or a world's server thread).
pub(super) struct GameHost<'a, 'g> {
    pub(super) game: &'a mut super::Game<'g>,
}

/// A command typed by a player or the console, before vanilla's dispatcher. The client also reaches
/// `native_command` before sending a command to its server, so only a server-side game claims it.
pub(super) fn on_command(game: &mut super::Game<'_>, command: &str, ran: &mut bool) {
    if game.native_window() || RUNNING_VANILLA.with_borrow(|r| *r) {
        return;
    }
    // A console command may come before the first tick: start Essentials here then.
    let dir = data_dir(game);
    ESS.with_borrow_mut(|slot| {
        if slot.is_none() && !STARTED.with_borrow(|s| *s) {
            *slot = Some(Ess::new(&dir));
            STARTED.with_borrow_mut(|s| *s = true);
        }
    });
    let claims = ESS.with_borrow(|ess| ess.as_ref().is_some_and(|e| e.claims(command)));
    if !claims {
        return;
    }
    let sender = match game.lan_acting_uuid {
        Some(uuid) => Sender::Player(uuid),
        None => Sender::Console,
    };
    QUEUE.with_borrow_mut(|q| q.push((sender, command.to_string())));
    *ran = true;
}

/// Once per server tick: start Essentials, notice joins and quits, run queued commands, tick.
pub(super) fn on_tick(game: &mut super::Game<'_>) {
    if game.native_window() {
        return;
    }
    let started = STARTED.with_borrow(|s| *s);
    let Some(mut ess) = ESS.with_borrow_mut(Option::take).or_else(|| (!started).then(|| Ess::new(&data_dir(game)))) else {
        return;
    };
    STARTED.with_borrow_mut(|s| *s = true);
    {
        let mut host = GameHost { game: &mut *game };
        let online = host.online();
        let before = ONLINE.with_borrow(|o| o.clone());
        for p in online.iter().filter(|p| !before.contains(p)) {
            ess.on_join(&mut host, *p);
        }
        for p in before.iter().filter(|p| !online.contains(p)) {
            ess.on_quit(&mut host, *p);
        }
        ONLINE.with_borrow_mut(|o| *o = online);
        for (sender, line) in QUEUE.with_borrow_mut(std::mem::take) {
            ess.dispatch(&mut host, sender, &line);
        }
        ess.on_tick(&mut host);
    }
    ESS.with_borrow_mut(|slot| *slot = Some(ess));
}

/// Runs `f` with the running Essentials and a host over `game` (for area hooks: chat, death, blocks...).
/// None before Essentials started or while another Essentials call is in progress.
pub(super) fn with_ess<T>(game: &mut super::Game<'_>, f: impl FnOnce(&mut Ess, &mut GameHost) -> T) -> Option<T> {
    if game.native_window() {
        return None;
    }
    let mut ess = ESS.with_borrow_mut(Option::take)?;
    let out = {
        let mut host = GameHost { game: &mut *game };
        f(&mut ess, &mut host)
    };
    ESS.with_borrow_mut(|slot| *slot = Some(ess));
    Some(out)
}

/// `<server>/essentials` on a dedicated server, `<world>/essentials` for a world's own server.
fn data_dir(game: &super::Game<'_>) -> std::path::PathBuf {
    if game.options.dedicated.is_some() {
        std::env::current_dir().unwrap_or_default().join("essentials")
    } else {
        game.world_root.join("essentials")
    }
}

/// Styled spans as a chat component (JSON, as the protocol's NBT form expects).
pub(super) fn component(spans: &[Span]) -> Value {
    let extra: Vec<Value> = spans.iter().map(|s| span_json(&s.text, &s.style)).collect();
    match extra.len() {
        0 => json!({"text": ""}),
        _ => json!({"text": "", "extra": extra}),
    }
}

fn span_json(text: &str, style: &Style) -> Value {
    let mut o = serde_json::Map::new();
    o.insert("text".into(), json!(text));
    if let Some(c) = &style.color {
        o.insert("color".into(), json!(c));
    }
    for (key, on) in [("bold", style.bold), ("italic", style.italic), ("underlined", style.underlined), ("strikethrough", style.strikethrough), ("obfuscated", style.obfuscated)] {
        if on {
            o.insert(key.into(), json!(true));
        }
    }
    // Chat text never inherits italics from item names: say so only when asked.
    if let Some((action, value)) = &style.click {
        let field = match action.as_str() {
            "open_url" => "url",
            "change_page" => "page",
            "copy_to_clipboard" => "value",
            _ => "command",
        };
        o.insert("click_event".into(), json!({"action": action, field: value}));
    }
    if let Some(hover) = &style.hover {
        o.insert("hover_event".into(), json!({"action": "show_text", "value": hover}));
    }
    Value::Object(o)
}

pub(super) fn nbt(spans: &[Span]) -> horizon_world::nbt::Tag {
    horizon_client::lan::data::component_nbt(&component(spans))
}

impl GameHost<'_, '_> {
    pub(super) fn hub(&self) -> Option<&horizon_client::lan::hub::Hub> {
        self.game.lan.as_ref().map(|h| &h.hub)
    }
    pub(super) fn hub_mut(&mut self) -> Option<&mut horizon_client::lan::hub::Hub> {
        self.game.lan.as_mut().map(|h| &mut h.hub)
    }
    /// A placed player's index in the hub.
    pub(super) fn index(&self, player: Uuid) -> Option<usize> {
        self.hub()?.guests.iter().position(|g| g.uuid == player && g.placed)
    }
    pub(super) fn guest(&self, player: Uuid) -> Option<&horizon_client::lan::guest::Guest> {
        self.hub()?.guests.iter().find(|g| g.uuid == player && g.placed)
    }
    pub(super) fn guest_mut(&mut self, player: Uuid) -> Option<&mut horizon_client::lan::guest::Guest> {
        self.hub_mut()?.guests.iter_mut().find(|g| g.uuid == player && g.placed)
    }
    pub(super) fn conn(&self, player: Uuid) -> Option<u32> {
        self.guest(player).map(|g| g.conn)
    }
    /// Runs a vanilla command line with the hook kept out (Essentials overrides several vanilla names).
    pub(super) fn vanilla<T>(&mut self, f: impl FnOnce(&mut super::Game<'_>) -> T) -> T {
        RUNNING_VANILLA.with_borrow_mut(|r| *r = true);
        let out = f(self.game);
        RUNNING_VANILLA.with_borrow_mut(|r| *r = false);
        out
    }
}

impl Host for GameHost<'_, '_> {
    // ----- Core (this file) ---------------------------------------------------------------------------
    fn now_millis(&mut self) -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
    }
    fn online(&mut self) -> Vec<Uuid> {
        self.hub().map(|h| h.guests.iter().filter(|g| g.placed).map(|g| g.uuid).collect()).unwrap_or_default()
    }
    fn name(&mut self, player: Uuid) -> Option<String> {
        self.hub()?.guests.iter().find(|g| g.uuid == player).map(|g| g.name.clone())
    }
    fn lookup(&mut self, name: &str) -> Option<(Uuid, String)> {
        if let Some(g) = self.hub().and_then(|h| h.guests.iter().find(|g| g.name.eq_ignore_ascii_case(name))) {
            return Some((g.uuid, g.name.clone()));
        }
        self.admin_lookup(name)
    }
    fn op_level(&mut self, player: Uuid) -> u8 {
        self.guest(player).map_or(0, |g| g.permission)
    }
    fn address(&mut self, player: Uuid) -> Option<String> {
        self.guest(player).map(|g| g.address.ip().to_string())
    }
    fn kick(&mut self, player: Uuid, reason: &[Span]) {
        let tag = nbt(reason);
        if let (Some(conn), Some(hub)) = (self.conn(player), self.hub_mut()) {
            hub.kick(conn, tag);
        }
    }
    fn send(&mut self, to: Sender, text: &[Span]) {
        match to {
            Sender::Console => {
                let line = crate::game::ess_text::plain(text);
                if self.game.options.dedicated.is_some() {
                    super::dedicated::log("INFO", &line);
                } else {
                    ::log::info!("[Essentials] {line}");
                }
            }
            Sender::Player(p) => {
                let content = nbt(text);
                if let (Some(conn), Some(hub)) = (self.conn(p), self.hub()) {
                    hub.send_to(conn, &C::SystemChat(play::SystemChat { content, overlay: false }));
                }
            }
        }
    }
    fn action_bar(&mut self, player: Uuid, text: &[Span]) {
        let content = nbt(text);
        if let (Some(conn), Some(hub)) = (self.conn(player), self.hub()) {
            hub.send_to(conn, &C::SystemChat(play::SystemChat { content, overlay: true }));
        }
    }
    fn location(&mut self, player: Uuid) -> Option<Loc> {
        let g = self.guest(player)?;
        Some(Loc { world: g.dimension.clone(), x: g.position[0], y: g.position[1], z: g.position[2], yaw: g.yaw, pitch: g.pitch })
    }
    fn teleport(&mut self, player: Uuid, to: &Loc) -> bool {
        let Some(i) = self.index(player) else { return false };
        let at = [to.x, to.y, to.z];
        let same = self.hub().is_some_and(|h| h.guests[i].dimension == to.world);
        if same {
            if let Some(hub) = self.hub_mut() {
                hub.guests[i].yaw = to.yaw;
                hub.guests[i].pitch = to.pitch;
                hub.teleport(i, Some(at));
            }
        } else {
            let conn = self.hub().map(|h| h.guests[i].conn).unwrap_or_default();
            self.game.level_teleports.push(super::server_travel::LevelTeleport {
                conn,
                dim: super::Dim::from_id(&to.world),
                at,
                rotation: Some([to.yaw, to.pitch]),
            });
        }
        true
    }
    fn console_command(&mut self, line: &str) -> (bool, Vec<String>) {
        let line = line.trim_start_matches('/').to_string();
        self.vanilla(|g| {
            g.chat_capture = Some(Vec::new());
            let ok = g.as_console(|g| g.run_command(&line));
            let lines = g.chat_capture.take().unwrap_or_default().iter().map(|t| t.plain()).collect();
            (ok, lines)
        })
    }
    fn player_command(&mut self, player: Uuid, line: &str, op: bool) -> (bool, Vec<String>) {
        let Some(g) = self.guest(player) else { return (false, Vec::new()) };
        let command = super::lan_actions::GuestCommand {
            conn: g.conn,
            command: line.trim_start_matches('/').to_string(),
            permission: if op { 4 } else { g.permission },
            integrated: self.game.options.dedicated.is_none(),
            quiet: false,
        };
        let ok = self.vanilla(|g| g.run_guest_command(command));
        (ok, Vec::new())
    }
    fn server_dir(&mut self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_default()
    }

    // ----- Area: tp (mixins/ess-tp/ess_glue_tp.rs) ----------------------------------------------------
    fn world_spawn(&mut self, world: &str) -> Option<Loc> {
        self.tp_world_spawn(world)
    }
    fn set_world_spawn(&mut self, at: &Loc) {
        self.tp_set_world_spawn(at)
    }
    fn bed_spawn(&mut self, player: Uuid) -> Option<Loc> {
        self.tp_bed_spawn(player)
    }
    fn block(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        self.tp_block(world, pos)
    }
    fn set_block(&mut self, world: &str, pos: [i32; 3], state: &str) -> bool {
        self.tp_set_block(world, pos, state)
    }
    fn highest_y(&mut self, world: &str, x: i32, z: i32) -> Option<i32> {
        self.tp_highest_y(world, x, z)
    }
    fn height_range(&mut self, world: &str) -> (i32, i32) {
        self.tp_height_range(world)
    }
    fn target_block(&mut self, player: Uuid, max: f64) -> Option<[i32; 3]> {
        self.tp_target_block(player, max)
    }
    fn biome(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        self.tp_biome(world, pos)
    }

    // ----- Area: social (mixins/ess-social/ess_glue_social.rs) -------------------------------------------
    fn broadcast(&mut self, text: &[Span]) {
        self.social_broadcast(text)
    }
    fn chat_as(&mut self, player: Uuid, text: &[Span]) {
        self.social_chat_as(player, text)
    }
    fn set_display_name(&mut self, player: Uuid, name: Option<&[Span]>) {
        self.social_set_display_name(player, name)
    }
    fn set_tab_name(&mut self, player: Uuid, name: Option<&[Span]>) {
        self.social_set_tab_name(player, name)
    }
    fn set_motd(&mut self, motd: &[Span]) {
        self.social_set_motd(motd)
    }

    // ----- Area: admin (mixins/ess-admin/ess_glue_admin.rs) ---------------------------------------------
    fn set_hidden(&mut self, player: Uuid, hidden: bool, seers: &[Uuid]) {
        self.admin_set_hidden(player, hidden, seers)
    }
    fn bans(&mut self, ip: bool) -> Vec<ess_core::Ban> {
        self.admin_bans(ip)
    }
    fn ban(&mut self, ip: bool, ban: &ess_core::Ban) {
        self.admin_ban(ip, ban)
    }
    fn unban(&mut self, ip: bool, target: &str) -> bool {
        self.admin_unban(ip, target)
    }
    fn stats(&mut self) -> ServerStats {
        self.admin_stats()
    }
    fn max_players(&mut self) -> usize {
        self.admin_max_players()
    }
    fn statistic(&mut self, player: Uuid, stat: &str) -> i64 {
        self.admin_statistic(player, stat)
    }

    // ----- Area: items (mixins/ess-items/ess_glue_items.rs) ----------------------------------------------
    fn inventory(&mut self, player: Uuid) -> Vec<Option<Item>> {
        self.items_inventory(player)
    }
    fn set_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
        self.items_set_slot(player, slot, item)
    }
    fn held_slot(&mut self, player: Uuid) -> usize {
        self.items_held_slot(player)
    }
    fn ender_chest(&mut self, player: Uuid) -> Vec<Option<Item>> {
        self.items_ender_chest(player)
    }
    fn set_ender_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
        self.items_set_ender_slot(player, slot, item)
    }
    fn open_container(&mut self, player: Uuid, container: &ess_core::Container) -> bool {
        self.items_open_container(player, container)
    }
    fn open_station(&mut self, player: Uuid, station: Station) -> bool {
        self.items_open_station(player, station)
    }
    fn close_container(&mut self, player: Uuid) {
        self.items_close_container(player)
    }
    fn drop_item(&mut self, at: &Loc, item: &Item) {
        self.items_drop_item(at, item)
    }
    fn item_ids(&mut self) -> Vec<String> {
        self.items_item_ids()
    }
    fn max_stack(&mut self, item: &str) -> i32 {
        self.items_max_stack(item)
    }
    fn max_damage(&mut self, item: &str) -> i32 {
        self.items_max_damage(item)
    }
    fn enchantments(&mut self) -> Vec<(String, i32)> {
        self.items_enchantments()
    }

    // ----- Area: economy (mixins/ess-economy/ess_glue_economy.rs) ----------------------------------------
    fn sign_lines(&mut self, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
        self.economy_sign_lines(world, pos)
    }
    fn set_sign_lines(&mut self, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
        self.economy_set_sign_lines(world, pos, lines)
    }

    // ----- Area: world (mixins/ess-world/ess_glue_world.rs) ----------------------------------------------
    fn game_mode(&mut self, player: Uuid) -> GameMode {
        self.world_game_mode(player)
    }
    fn set_game_mode(&mut self, player: Uuid, mode: GameMode) {
        self.world_set_game_mode(player, mode)
    }
    fn health(&mut self, player: Uuid) -> (f64, f64) {
        self.world_health(player)
    }
    fn set_health(&mut self, player: Uuid, health: f64) {
        self.world_set_health(player, health)
    }
    fn food(&mut self, player: Uuid) -> (i32, f32) {
        self.world_food(player)
    }
    fn set_food(&mut self, player: Uuid, food: i32, saturation: f32) {
        self.world_set_food(player, food, saturation)
    }
    fn set_fire_ticks(&mut self, player: Uuid, ticks: i32) {
        self.world_set_fire_ticks(player, ticks)
    }
    fn fire_ticks(&mut self, player: Uuid) -> i32 {
        self.world_fire_ticks(player)
    }
    fn set_freeze_ticks(&mut self, player: Uuid, ticks: i32) {
        self.world_set_freeze_ticks(player, ticks)
    }
    fn exp(&mut self, player: Uuid) -> i64 {
        self.world_exp(player)
    }
    fn set_exp(&mut self, player: Uuid, total: i64) {
        self.world_set_exp(player, total)
    }
    fn flying(&mut self, player: Uuid) -> bool {
        self.world_flying(player)
    }
    fn set_allow_flight(&mut self, player: Uuid, allow: bool) {
        self.world_set_allow_flight(player, allow)
    }
    fn allow_flight(&mut self, player: Uuid) -> bool {
        self.world_allow_flight(player)
    }
    fn set_flying(&mut self, player: Uuid, flying: bool) {
        self.world_set_flying(player, flying)
    }
    fn set_fly_speed(&mut self, player: Uuid, speed: f32) {
        self.world_set_fly_speed(player, speed)
    }
    fn set_walk_speed(&mut self, player: Uuid, speed: f32) {
        self.world_set_walk_speed(player, speed)
    }
    fn set_invulnerable(&mut self, player: Uuid, invulnerable: bool) {
        self.world_set_invulnerable(player, invulnerable)
    }
    fn set_player_time(&mut self, player: Uuid, time: Option<(i64, bool)>) {
        self.world_set_player_time(player, time)
    }
    fn set_player_weather(&mut self, player: Uuid, downfall: Option<bool>) {
        self.world_set_player_weather(player, downfall)
    }
    fn entity_types(&mut self) -> Vec<String> {
        self.world_entity_types()
    }
    fn world_time(&mut self, world: &str) -> i64 {
        self.world_world_time(world)
    }
    fn set_world_time(&mut self, world: &str, time: i64) {
        self.world_set_world_time(world, time)
    }
    fn weather(&mut self, world: &str) -> (bool, bool) {
        self.world_weather(world)
    }
    fn set_weather(&mut self, world: &str, rain: bool, thunder: bool, duration: Option<i32>) {
        self.world_set_weather(world, rain, thunder, duration)
    }
    fn spawn_entity(&mut self, kind: &str, at: &Loc, snbt: Option<&str>) -> bool {
        self.world_spawn_entity(kind, at, snbt)
    }
    fn strike_lightning(&mut self, at: &Loc, effect_only: bool) {
        self.world_strike_lightning(at, effect_only)
    }
    fn remove_entities(&mut self, world: &str, center: Option<&Loc>, radius: Option<f64>, kinds: &[String]) -> usize {
        self.world_remove_entities(world, center, radius, kinds)
    }

    // ----- Area extensions: Host methods an area added (delegate to its ess_glue_<area> functions) ----
    // ---- area tp ----
    fn is_dead(&mut self, player: Uuid) -> bool {
        self.tp_is_dead(player)
    }
    // ---- end tp ----
    // ---- area social ----
    // ---- end social ----
    // ---- area admin ----
    fn whitelisted(&mut self, player: Uuid) -> bool {
        self.admin_whitelisted(player)
    }
    fn first_played(&mut self, player: Uuid) -> Option<i64> {
        self.admin_first_played(player)
    }
    fn speeds(&mut self, player: Uuid) -> (f32, f32) {
        self.admin_speeds(player)
    }
    // ---- end admin ----
    // ---- area items ----
    // ---- end items ----
    // ---- area economy ----
    fn sneaking(&mut self, player: Uuid) -> bool {
        self.economy_sneaking(player)
    }
    fn item_is_block(&mut self, item: &str) -> bool {
        self.economy_item_is_block(item)
    }
    // ---- end economy ----
    // ---- area world ----
    fn speeds(&mut self, player: Uuid) -> (f32, f32) {
        self.world_speeds(player)
    }
    fn set_statistic(&mut self, player: Uuid, stat: &str, value: i64) {
        self.world_set_statistic(player, stat, value)
    }
    fn http_request(&mut self, id: u64, method: &str, url: &str, headers: &[(String, String)], body: Option<&str>) {
        self.world_http_request(id, method, url, headers, body)
    }
    fn http_poll(&mut self) -> Vec<(u64, u16, String)> {
        self.world_http_poll()
    }
    fn tcp_open(&mut self, id: u64, host: &str, port: u16, tls: bool) {
        self.world_tcp_open(id, host, port, tls)
    }
    fn tcp_send(&mut self, id: u64, data: &[u8]) {
        self.world_tcp_send(id, data)
    }
    fn tcp_close(&mut self, id: u64) {
        self.world_tcp_close(id)
    }
    fn tcp_poll(&mut self) -> Vec<(u64, Vec<u8>)> {
        self.world_tcp_poll()
    }
    // ---- end world ----
}
