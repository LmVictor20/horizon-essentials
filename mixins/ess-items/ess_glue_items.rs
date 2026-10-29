// horizon-essentials glue for area `items` (game only): `crate::game::ess_glue_items`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here), and the
// handlers of this area's hooks (hook_*.rs next to this file).
//
// Inventories: Bukkit slots 0-35 are Guest.inventory (0-8 the hotbar), 36-39 boots..helmet are Guest.armor[3..0],
// 40 the offhand. Items convert through Horizon's `components` compound (decode_components/encode_components),
// keys without the `minecraft:` namespace on Essentials' side; custom names travel as `§`-coded strings.
//
// Plugin containers (disposal, others' ender chests, admin's /invsee) use the vehicle-chest trick: a stand-in
// chest block entity under a key at VEHICLE_KEY_Y, opened with Slots::Block(key); chest_vehicle_still_valid
// answers true for our keys (hook). Clicks are checked against a policy the pure side computes once per open
// (the hub is lent out while packets run, so Essentials can't be asked then); closes are noticed each tick.
// Portable workstations: the crafting table through lan_open_crafting at a VEHICLE_KEY_Y position
// (lan_table_valid hook), the others through as_guest + open_station, hidden from lan_stations_tick's
// block checks while it runs (hooks on lan_stations_tick / lan_tables_tick).

use super::ess_glue::GameHost;
use crate::game::ess_core::*;
use horizon_client::containers::BlockEntity;
use horizon_client::items::ItemStack;
use horizon_client::lan::menu::{Slots, VEHICLE_KEY_Y};
use horizon_protocol::packets::play::{self, Clientbound as C};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// One open plugin container.
struct Virtual {
    player: Uuid,
    conn: u32,
    key: (i32, i32, i32),
    dim: String,
    tag: String,
    window: i32,
    size: usize,
    /// The player's own ender chest (Slots::Ender): nothing to report on close.
    own_ender: bool,
    /// Allowed raw slots (None until the pure side answered: everything allowed meanwhile is wrong, so
    /// clicks wait for it by being refused).
    policy: Option<Vec<bool>>,
    reported_open: bool,
}

/// A portable workstation (anvil, loom...) open for a guest: its window.
struct Portable {
    window: i32,
}

thread_local! {
    static VIRTUALS: RefCell<Vec<Virtual>> = const { RefCell::new(Vec::new()) };
    static NEXT_KEY: RefCell<i32> = const { RefCell::new(0) };
    static PORTABLE: RefCell<HashMap<u32, Portable>> = RefCell::new(HashMap::new());
    static STASHED: RefCell<Vec<(u32, super::lan_stations::GuestStation)>> = const { RefCell::new(Vec::new()) };
}

fn ns(id: &str) -> String {
    if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") }
}

/// Essentials' item as Horizon's stack.
pub(super) fn to_stack(item: &Item) -> Option<ItemStack> {
    if item.count <= 0 || item.id.is_empty() || item.id == "minecraft:air" {
        return None;
    }
    let mut map = Map::new();
    if let Some(m) = item.components.as_object() {
        for (k, v) in m {
            let key = ns(k);
            let v = if key == "minecraft:custom_name" { Value::String(crate::game::ess_items::component_section(v)) } else { v.clone() };
            map.insert(key, v);
        }
    }
    let (damage, extra) = if map.is_empty() {
        (0, None)
    } else {
        let tag = horizon_client_core::scoreboard::json_to_tag(&Value::Object(map));
        horizon_client::item_data::decode_components(Some(&tag))
    };
    Some(ItemStack { item: Arc::from(ns(&item.id).as_str()), count: item.count as u32, damage, extra })
}

/// Horizon's stack as Essentials' item.
pub(super) fn from_stack(stack: &ItemStack) -> Item {
    let mut comps = Map::new();
    if let Some(horizon_world::nbt::Tag::Compound(c)) = horizon_client::item_data::encode_components(stack) {
        for (k, v) in c.iter() {
            let key = k.to_string_lossy();
            let bare = key.strip_prefix("minecraft:").unwrap_or(&key).to_string();
            let mut value = horizon_client_core::scoreboard::tag_to_json(v);
            if bare == "custom_name" {
                if let Some(s) = value.as_str() {
                    value = crate::game::ess_items::legacy_component(s);
                }
            }
            comps.insert(bare, value);
        }
    }
    Item { id: stack.item.to_string(), count: stack.count as i32, components: Value::Object(comps) }
}

fn opt_item(s: &Option<ItemStack>) -> Option<Item> {
    s.as_ref().filter(|s| s.count > 0).map(from_stack)
}

/// Bukkit slot -> the guest's stack.
fn slot_ref(g: &horizon_client::lan::guest::Guest, slot: usize) -> Option<&Option<ItemStack>> {
    match slot {
        0..=35 => g.inventory.get(slot),
        36..=39 => g.armor.get(39 - slot),
        40 => Some(&g.offhand),
        _ => None,
    }
}

impl GameHost<'_, '_> {
    /// Runs `f` with `dim`'s level in place (the level of a guest or location), then puts the level back.
    fn in_level<T>(&mut self, dim: &str, f: impl FnOnce(&mut super::Game<'_>) -> T) -> T {
        let target = super::Dim::from_id(dim);
        let back = self.game.enter_command_level(target);
        let out = f(self.game);
        if let Some(back) = back {
            if let Err(e) = self.game.enter_level(None, back) {
                ::log::error!("[Essentials] could not put {} back: {e}", back.id());
            }
        }
        out
    }

    fn send_inventory_of(&mut self, player: Uuid) {
        if let (Some(i), Some(hub)) = (self.index(player), self.hub_mut()) {
            hub.send_inventory(i);
        }
    }

    pub(super) fn items_inventory(&mut self, player: Uuid) -> Vec<Option<Item>> {
        let Some(g) = self.guest(player) else { return vec![None; 41] };
        (0..41).map(|s| slot_ref(g, s).and_then(opt_item)).collect()
    }
    pub(super) fn items_set_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
        let stack = item.as_ref().and_then(to_stack);
        let Some(g) = self.guest_mut(player) else { return };
        match slot {
            0..=35 => {
                if g.inventory.len() <= slot {
                    g.inventory.resize(36, None);
                }
                g.inventory[slot] = stack;
            }
            36..=39 => g.armor[39 - slot] = stack,
            40 => g.offhand = stack,
            _ => return,
        }
        self.send_inventory_of(player);
    }
    pub(super) fn items_held_slot(&mut self, player: Uuid) -> usize {
        self.guest(player).map_or(0, |g| g.selected)
    }
    pub(super) fn items_ender_chest(&mut self, player: Uuid) -> Vec<Option<Item>> {
        let Some(g) = self.guest(player) else { return vec![None; 27] };
        let mut out: Vec<Option<Item>> = g.ender_items.iter().map(opt_item).collect();
        out.resize(27, None);
        out
    }
    pub(super) fn items_set_ender_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
        let stack = item.as_ref().and_then(to_stack);
        if let Some(g) = self.guest_mut(player) {
            if g.ender_items.len() < 27 {
                g.ender_items.resize(27, None);
            }
            if slot < g.ender_items.len() {
                g.ender_items[slot] = stack;
            }
        }
    }

    /// Closes what the guest has open on the game's side (a station, a crafting table, a container's lid).
    fn close_open_menus(&mut self, conn: u32) {
        let Some(mut lan) = self.game.lan.take() else { return };
        self.game.lan_station_closed(&mut lan, conn, None);
        self.game.lan_crafting_closed(&mut lan, conn, None);
        self.game.guest_container_closed(&mut lan, conn);
        self.game.lan = Some(lan);
        PORTABLE.with_borrow_mut(|p| p.remove(&conn));
    }

    pub(super) fn items_open_container(&mut self, player: Uuid, container: &Container) -> bool {
        let Some(g) = self.guest(player) else { return false };
        let (conn, dim) = (g.conn, g.dimension.clone());
        let rows = container.rows.clamp(1, 6);
        let size = rows as usize * 9;
        let mut items: Vec<Option<ItemStack>> = container.items.iter().take(size).map(|i| i.as_ref().and_then(to_stack)).collect();
        items.resize(size, None);
        self.items_close_container(player);
        self.close_open_menus(conn);
        let id = NEXT_KEY.with_borrow_mut(|n| {
            *n += 1;
            -*n
        });
        let key = (i32::MAX - 16, VEHICLE_KEY_Y, id);
        let title = super::ess_glue::nbt(&crate::game::ess_text::parse_mini(&container.title, &crate::game::ess_text::Palette::default()));
        let menu = format!("minecraft:generic_9x{rows}");
        let opened = self.in_level(&dim, |game| {
            game.world.block_entities.insert(key, BlockEntity::Chest(items.clone()));
            let Some(lan) = game.lan.as_mut() else { return None };
            lan.hub.open_container(conn, &menu, title, key, Slots::Block(key), items);
            let window = lan.hub.guest(conn).and_then(|g| g.menu.as_ref().filter(|m| m.pos == Some(key)).map(|m| m.window));
            if window.is_none() {
                game.world.block_entities.remove(&key);
            }
            window
        });
        let Some(window) = opened else { return false };
        VIRTUALS.with_borrow_mut(|v| {
            v.push(Virtual { player, conn, key, dim, tag: container.tag.clone(), window, size, own_ender: false, policy: None, reported_open: false })
        });
        true
    }

    pub(super) fn items_open_station(&mut self, player: Uuid, station: Station) -> bool {
        let Some(g) = self.guest(player) else { return false };
        let (conn, dim) = (g.conn, g.dimension.clone());
        let pos = [g.position[0].floor() as i32, g.position[1].floor() as i32, g.position[2].floor() as i32];
        self.items_close_container(player);
        self.close_open_menus(conn);
        match station {
            Station::EnderChest => {
                let id = NEXT_KEY.with_borrow_mut(|n| {
                    *n += 1;
                    -*n
                });
                let key = (i32::MAX - 16, VEHICLE_KEY_Y, id);
                let items = g_ender(self, player);
                let window = self.hub_mut().and_then(|hub| {
                    hub.open_container(conn, "minecraft:generic_9x3", horizon_protocol::text::translatable("container.enderchest", &[]), key, Slots::Ender, items);
                    hub.guest(conn).and_then(|g| g.menu.as_ref().map(|m| m.window))
                });
                let Some(window) = window else { return false };
                VIRTUALS.with_borrow_mut(|v| {
                    v.push(Virtual { player, conn, key, dim, tag: String::new(), window, size: 27, own_ender: true, policy: Some(vec![true; 63]), reported_open: true })
                });
                true
            }
            Station::Crafting => self.in_level(&dim, |game| {
                let Some(mut lan) = game.lan.take() else { return false };
                game.lan_open_crafting(&mut lan, conn, [pos[0], VEHICLE_KEY_Y, pos[2]]);
                game.lan = Some(lan);
                true
            }),
            other => {
                let block = match other {
                    Station::Anvil => "minecraft:anvil",
                    Station::Cartography => "minecraft:cartography_table",
                    Station::Grindstone => "minecraft:grindstone",
                    Station::Loom => "minecraft:loom",
                    Station::Smithing => "minecraft:smithing_table",
                    _ => "minecraft:stonecutter",
                };
                self.in_level(&dim, |game| {
                    let Some(mut lan) = game.lan.take() else { return false };
                    game.as_guest(&mut lan, conn, |g| {
                        g.open_station(pos, block);
                    });
                    let window = lan.hub.guest(conn).map(|g| g.next_window);
                    let open = lan.play.get(&conn).is_some_and(|p| p.station.is_some());
                    game.lan = Some(lan);
                    if let (Some(window), true) = (window, open) {
                        PORTABLE.with_borrow_mut(|p| p.insert(conn, Portable { window }));
                    }
                    open
                })
            }
        }
    }

    pub(super) fn items_close_container(&mut self, player: Uuid) {
        let Some(conn) = self.conn(player) else { return };
        if let Some(hub) = self.hub_mut() {
            hub.close_container(conn);
        }
        self.close_open_menus(conn);
    }

    pub(super) fn items_drop_item(&mut self, at: &Loc, item: &Item) {
        let Some(stack) = to_stack(item) else { return };
        let pos = [at.x, at.y + 0.5, at.z];
        self.in_level(&at.world, |game| game.spawn_item(stack, pos, [0.0, 0.2, 0.0], 10));
    }

    pub(super) fn items_item_ids(&mut self) -> Vec<String> {
        let items = self.game.res.items();
        let mut out: Vec<String> = if items.registry_order.is_empty() { items.items.keys().map(|k| k.to_string()).collect() } else { items.registry_order.iter().map(|k| k.to_string()).collect() };
        if !out.iter().any(|i| i == "minecraft:air") {
            out.push("minecraft:air".into());
        }
        out
    }
    pub(super) fn items_max_stack(&mut self, item: &str) -> i32 {
        self.game.res.items().get(&ns(item)).map_or(64, |i| i.max_stack.max(1) as i32)
    }
    pub(super) fn items_max_damage(&mut self, item: &str) -> i32 {
        self.game.res.items().get(&ns(item)).map_or(0, |i| i.max_damage as i32)
    }
    pub(super) fn items_enchantments(&mut self) -> Vec<(String, i32)> {
        self.game.res.items().enchantments.list.iter().map(|e| (e.id.to_string(), e.max_level as i32)).collect()
    }
    pub(super) fn items_can_enchant(&mut self, item: &str, enchantment: &str) -> bool {
        let items = self.game.res.items();
        items.enchantments.can_enchant(items, &ns(enchantment), &ns(item))
    }

    /// Horizon's recipes as vanilla recipe JSON (shaped cells named a..i as CraftBukkit does).
    pub(super) fn items_recipes(&mut self) -> Vec<Value> {
        use horizon_client::crafting::{Recipe, Special};
        let recipes = self.game.res.recipes();
        let ing = |i: &horizon_client::crafting::Ingredient| json!(i.1);
        let result = |t: &horizon_client::crafting::Template| t.item.as_ref().map(|id| json!({"id": id.to_string(), "count": t.count.max(1)}));
        let mut out = Vec::new();
        for (id, r) in &recipes.crafting {
            let id = ns(id);
            match r {
                Recipe::Shaped { width, height, cells, result: res } => {
                    let Some(res) = result(res) else { continue };
                    let mut key = Map::new();
                    let mut pattern = Vec::new();
                    for y in 0..*height {
                        let mut row = String::new();
                        for x in 0..*width {
                            let n = y * width + x;
                            match cells.get(n).and_then(Option::as_ref) {
                                Some(c) => {
                                    let ch = char::from(b'a' + n as u8);
                                    key.insert(ch.to_string(), ing(c));
                                    row.push(ch);
                                }
                                None => row.push(' '),
                            }
                        }
                        pattern.push(row);
                    }
                    out.push(json!({"id": id, "type": "minecraft:crafting_shaped", "pattern": pattern, "key": key, "result": res}));
                }
                Recipe::Shapeless { ingredients, result: res } => {
                    let Some(res) = result(res) else { continue };
                    out.push(json!({"id": id, "type": "minecraft:crafting_shapeless", "ingredients": ingredients.iter().map(ing).collect::<Vec<_>>(), "result": res}));
                }
                Recipe::Special(Special::Transmute { input, material, result: res, .. }) => {
                    let Some(res) = result(res) else { continue };
                    out.push(json!({"id": id, "type": "minecraft:crafting_transmute", "input": ing(input), "material": ing(material), "result": res}));
                }
                Recipe::Special(_) => {}
            }
        }
        for c in &recipes.cooking {
            out.push(json!({"id": ns(&c.id), "type": ns(&c.kind), "ingredient": ing(&c.ingredient), "result": {"id": c.result.item.to_string(), "count": c.result.count}}));
        }
        out
    }

    /// Plugin containers whose menu is gone (closed, replaced, or the player left): their final items.
    pub(super) fn items_closed_containers(&mut self) -> Vec<(Uuid, String, Vec<Option<Item>>)> {
        let all = VIRTUALS.with_borrow_mut(std::mem::take);
        let mut keep = Vec::new();
        let mut out = Vec::new();
        for v in all {
            let open = self.hub().and_then(|h| h.guests.iter().find(|g| g.uuid == v.player)).is_some_and(|g| g.menu.as_ref().is_some_and(|m| m.pos == Some(v.key) && m.window == v.window));
            if open {
                keep.push(v);
                continue;
            }
            if v.own_ender {
                continue;
            }
            let key = v.key;
            let stacks = self.in_level(&v.dim, |game| match game.world.block_entities.remove(&key) {
                Some(BlockEntity::Chest(items)) => items,
                _ => Vec::new(),
            });
            let mut items: Vec<Option<Item>> = stacks.iter().map(opt_item).collect();
            items.resize(v.size, None);
            out.push((v.player, v.tag, items));
        }
        VIRTUALS.with_borrow_mut(|v| {
            keep.extend(v.drain(..));
            *v = keep;
        });
        out
    }

    pub(super) fn items_opened_containers(&mut self) -> Vec<(Uuid, String, usize)> {
        VIRTUALS.with_borrow_mut(|all| {
            all.iter_mut()
                .filter(|v| !v.reported_open)
                .map(|v| {
                    v.reported_open = true;
                    (v.player, v.tag.clone(), v.size + 36)
                })
                .collect()
        })
    }
    pub(super) fn items_set_container_policy(&mut self, player: Uuid, tag: &str, allowed: Vec<bool>) {
        VIRTUALS.with_borrow_mut(|all| {
            if let Some(v) = all.iter_mut().find(|v| v.player == player && v.tag == tag) {
                v.policy = Some(allowed);
            }
        });
    }
}

fn g_ender(host: &GameHost, player: Uuid) -> Vec<Option<ItemStack>> {
    let mut items = host.guest(player).map(|g| g.ender_items.clone()).unwrap_or_default();
    items.resize(27, None);
    items
}

// ===== Hook handlers ==========================================================================================

/// `chest_vehicle_still_valid` (HEAD, cancellable): our stand-in keys stay valid.
pub(super) fn vehicle_valid(id: i32) -> Option<bool> {
    VIRTUALS.with_borrow(|v| v.iter().any(|x| x.key.2 == id)).then_some(true)
}

/// `lan_table_valid` (RETURN): a portable crafting table sits at VEHICLE_KEY_Y.
pub(super) fn table_valid(valid: bool, pos: [i32; 3]) -> bool {
    valid || pos[1] == VEHICLE_KEY_Y
}

/// `lan_stations_tick` (HEAD): the portable stations leave the guests' play state while the tick checks
/// their blocks...
pub(super) fn hide_portable_stations(host: &mut super::lan::LanHost) {
    let conns: Vec<(u32, i32)> = PORTABLE.with_borrow(|p| p.iter().map(|(c, s)| (*c, s.window)).collect());
    for (conn, window) in conns {
        let current = host.hub.guest(conn).map(|g| g.next_window);
        let Some(play) = host.play.get_mut(&conn) else {
            PORTABLE.with_borrow_mut(|p| p.remove(&conn));
            continue;
        };
        // Another window opened since: that station (if any) is a real one.
        if current != Some(window) || play.station.is_none() {
            PORTABLE.with_borrow_mut(|p| p.remove(&conn));
            continue;
        }
        if let Some(st) = play.station.take() {
            STASHED.with_borrow_mut(|s| s.push((conn, st)));
        }
    }
}

/// ...and come back right after (`lan_tables_tick`, HEAD: the next call in the same tick).
pub(super) fn restore_portable_stations(host: &mut super::lan::LanHost) {
    for (conn, st) in STASHED.with_borrow_mut(std::mem::take) {
        if let Some(play) = host.play.get_mut(&conn) {
            play.station = Some(st);
        }
    }
}

/// `lan_merchant_click` (HEAD, cancellable): the first check every container click goes through. A click in
/// one of our plugin containers that its policy refuses is answered here (Break(true): handled, nothing moves).
pub(super) fn on_click(game: &mut super::Game<'_>, host: &mut super::lan::LanHost, conn: u32, p: &play::ContainerClick) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else { return Continue(()) };
    let menu = host.hub.guests[gi].menu.as_ref().filter(|m| m.window == p.container).map(|m| m.pos);
    let Some(Some(pos)) = menu else { return Continue(()) };
    let allowed = VIRTUALS.with_borrow(|all| {
        all.iter().find(|v| v.conn == conn && v.key == pos && v.window == p.container).map(|v| match (&v.policy, usize::try_from(p.slot)) {
            (None, _) => false,
            (Some(_), Err(_)) => true,
            (Some(policy), Ok(slot)) => policy.get(slot).copied().unwrap_or(true) && (p.input != 1 || policy.iter().all(|a| *a)),
        })
    });
    if allowed == Some(false) {
        resend_container(game, host, gi);
        return Break(true);
    }
    Continue(())
}

fn recent(uuid: Uuid) -> bool {
    LAST_ACTION.with_borrow(|d| d.get(&uuid).is_some_and(|t| t.elapsed().as_millis() < 150))
}
fn mark(uuid: Uuid) {
    LAST_ACTION.with_borrow_mut(|d| d.insert(uuid, std::time::Instant::now()));
}

/// `guest_attack` (HEAD, cancellable): a player hitting another player with a `{player}` powertool runs it
/// instead of the hit.
pub(super) fn on_attack(game: &mut super::Game<'_>, host: &mut super::lan::LanHost, conn: u32, id: i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else { return Continue(()) };
    let uuid = host.hub.guests[gi].uuid;
    mark(uuid);
    let Some(defender) = host.hub.guests.iter().find(|g| g.entity_id == id && g.placed).map(|g| g.name.clone()) else { return Continue(()) };
    let g = &host.hub.guests[gi];
    let held = g.inventory.get(g.selected).and_then(opt_item);
    let used = super::ess_glue::with_ess(game, |ess, _| crate::game::ess_items::powertool_attack(ess, uuid, held.as_ref(), &defender)).unwrap_or(false);
    if used { Break(()) } else { Continue(()) }
}

/// `reset_attack_strength` (HEAD): a guest's punch (a left click in the air or starting to dig; not an attack,
/// not a dig going on) fires their powertool next tick, when the hub is back to read the held item.
pub(super) fn on_punch(game: &mut super::Game<'_>) {
    let Some(uuid) = game.lan_acting_uuid else { return };
    if game.mining.as_ref().is_some_and(|m| m.ticks > 0) || recent(uuid) {
        return;
    }
    mark(uuid);
    super::ess_glue::with_ess(game, |ess, _| {
        ess.scheduler.later(
            1,
            Box::new(move |ess, host| {
                crate::game::ess_items::on_left_click(ess, host, uuid);
            }),
        );
    });
}

thread_local! {
    static USES: RefCell<Vec<(Uuid, usize, Item, bool, bool)>> = const { RefCell::new(Vec::new()) };
    static LAST_ACTION: RefCell<HashMap<Uuid, std::time::Instant>> = RefCell::new(HashMap::new());
}

/// `under_spawn_protection` (HEAD): runs before a guest uses their held item on a block (and before a dig):
/// the held stack as it was, for unlimited items (checked next tick by the items area).
pub(super) fn before_use(host: &super::lan::LanHost, gi: usize) {
    let Some(g) = host.hub.guests.get(gi) else { return };
    if let Some(item) = g.inventory.get(g.selected).and_then(opt_item) {
        USES.with_borrow_mut(|u| u.push((g.uuid, g.selected, item, g.game_mode == 0, true)));
    }
}

impl GameHost<'_, '_> {
    pub(super) fn items_item_uses(&mut self) -> Vec<(Uuid, usize, Item, bool, bool)> {
        USES.with_borrow_mut(std::mem::take)
    }
}

/// A refused click: the client gets the container and its carried stack back as the server has them.
fn resend_container(game: &mut super::Game<'_>, host: &mut super::lan::LanHost, gi: usize) {
    let Some(menu) = host.hub.guests[gi].menu.as_ref() else { return };
    let (window, size, slots) = (menu.window, menu.size, menu.slots);
    let mut items = slots.read(&game.world, &host.hub.guests[gi]).unwrap_or_default();
    items.resize(size, None);
    for n in 0..36 {
        let i = if n < 27 { 9 + n } else { n - 27 };
        items.push(host.hub.guests[gi].inventory.get(i).cloned().flatten());
    }
    let net: Vec<_> = items.iter().map(|s| host.hub.stack(s)).collect();
    let carried = host.hub.guests[gi].carried.clone();
    let carried = host.hub.stack(&carried);
    let g = &mut host.hub.guests[gi];
    let Some(m) = g.menu.as_mut() else { return };
    m.state = (m.state + 1) & 0x7FFF;
    let state = m.state;
    m.sent = items;
    g.send(&C::ContainerSetContent(play::ContainerSetContent { container: window, state, items: net, carried }));
}
