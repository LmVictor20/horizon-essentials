// horizon-essentials glue for area `economy` (game only): `crate::game::ess_glue_economy`.
// The Host methods this area owns (signs, sneaking, blocks of items) over Horizon's Game and hub, and the
// handlers of this area's hooks, which feed every area's sign_change / interact / block_break listeners:
//
// - Right clicks: Horizon's packet handler (lan_guest_packet) holds macros, so it can't be hooked. A right click
//   on an unwaxed sign opens its editor (lan_open_sign_editor, hooked): on a sign with text that is the
//   interact event (run on the next tick, when the hub is reachable), and an Essentials sign never opens
//   its editor. Opening a container (lan_open_container,
//   hooked) is the interact event of chests, furnaces... ([Protection]).
// - Sign edits: lan_sign_update holds a macro too. The editor's opening records the sign's text; the update
//   marks the block entity (mark_block_entity_updated, hooked), and on the next tick the sign_change listeners
//   get the new lines: they may rewrite them, or refuse the edit (the old text comes back).
// - Breaking: break_block_as_player (hooked, cancellable). A guest's turn holds the hub, so the listeners get a
//   detached host: players' names, op levels and places as of the last tick, the level in place, and messages
//   delivered on the next tick (anything else is a no-op there; the [Trade] payout is a scheduled task).

use super::ess_glue::{GameHost, with_ess};
use crate::game::ess_core::*;
use horizon_client::containers::BlockEntity;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::ControlFlow;

thread_local! {
    /// (uuid, name, op level, location) of every placed player, as of the last tick.
    static PLAYERS: RefCell<Vec<(Uuid, String, u8, Loc)>> = const { RefCell::new(Vec::new()) };
    /// Messages a detached host sent, delivered on the next tick.
    static OUTBOX: RefCell<Vec<(Sender, Vec<Span>, bool)>> = const { RefCell::new(Vec::new()) };
    /// Open sign editors: (dimension, pos) -> (editor, the text before the edit).
    static EDITING: RefCell<HashMap<(String, [i32; 3]), (Uuid, [String; 4])>> = RefCell::new(HashMap::new());
    /// Right clicks on signs with text, for the next tick's interact listeners: (player, dimension, pos).
    static CLICKED: RefCell<Vec<(Uuid, String, [i32; 3])>> = const { RefCell::new(Vec::new()) };
    /// Finished edits to show the sign_change listeners: (editor, dimension, pos, old text).
    static EDITED: RefCell<Vec<(Uuid, String, [i32; 3], [String; 4])>> = const { RefCell::new(Vec::new()) };
}

fn dim_id(game: &super::Game<'_>) -> String {
    game.dim.id().to_string()
}
fn read_sign(game: &super::Game<'_>, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
    if super::Dim::from_id(world) != game.dim {
        return None;
    }
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Sign(sign)) => Some(sign.front.lines.clone()),
        _ => None,
    }
}
fn write_sign(game: &mut super::Game<'_>, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
    if super::Dim::from_id(world) != game.dim {
        return false;
    }
    let key = (pos[0], pos[1], pos[2]);
    let Some(BlockEntity::Sign(sign)) = game.world.block_entities.get_mut(&key) else { return false };
    sign.front.lines = lines.clone();
    game.mark_block_entity_updated(key);
    true
}
/// A block state as Essentials reads it: `minecraft:oak_sign[rotation=4,waterlogged=false]`.
fn state_string(game: &super::Game<'_>, pos: [i32; 3]) -> String {
    let id = game.world.block(pos[0], pos[1], pos[2]);
    let Some(state) = game.pack.assets.registry.state(id) else { return "minecraft:air".into() };
    let name = if state.name.contains(':') { state.name.clone() } else { format!("minecraft:{}", state.name) };
    if state.properties.is_empty() {
        return name;
    }
    let props: Vec<String> = state.properties.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{name}[{}]", props.join(","))
}

impl GameHost<'_, '_> {
    /// Sign text is plain strings in Horizon (sent as plain text components): Essentials' legacy `§` lines
    /// are stored as they are and vanilla clients draw their colours. Only the level in place is reachable.
    pub(super) fn economy_sign_lines(&mut self, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
        read_sign(self.game, world, pos)
    }
    pub(super) fn economy_set_sign_lines(&mut self, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
        write_sign(self.game, world, pos, lines)
    }
    pub(super) fn economy_sneaking(&mut self, player: Uuid) -> bool {
        self.guest(player).is_some_and(|g| g.sneaking)
    }
    pub(super) fn economy_item_is_block(&mut self, item: &str) -> bool {
        let id = if item.contains(':') { item.to_string() } else { format!("minecraft:{item}") };
        id != "minecraft:air" && self.game.pack.assets.registry.default_state(&id).is_some()
    }
}

// ----- Hooks -------------------------------------------------------------------------------------------------------
// While a packet handler runs, the hub is out of `game.lan` (held as `host`), so GameHost can't reach the
// players. Interactions that need them run on the next tick; decisions that must be made now use `Detached`.

/// HEAD of `lan_open_sign_editor` (cancellable): an Essentials sign keeps its editor shut and is used on the
/// next tick (interact listeners with the full host); another sign with text is that tick's interact event
/// too, and its editor opens. The text is remembered for the edit's sign_change event.
pub(super) fn on_open_sign_editor(game: &mut super::Game<'_>, host: &mut super::lan::LanHost, conn: u32, pos: [i32; 3], front: bool) -> ControlFlow<()> {
    if game.native_window() {
        return ControlFlow::Continue(());
    }
    let Some(g) = host.hub.guests.iter().find(|g| g.conn == conn) else { return ControlFlow::Continue(()) };
    let (uuid, world) = (g.uuid, g.dimension.clone());
    let Some(old) = read_sign(game, &world, pos) else { return ControlFlow::Continue(()) };
    if old.iter().any(|l| !l.is_empty()) {
        CLICKED.with_borrow_mut(|c| c.push((uuid, world.clone(), pos)));
        let essentials = with_ess(game, |ess, _| crate::game::ess_economy::is_essentials_sign(ess, &old[0])).unwrap_or(false);
        if essentials {
            return ControlFlow::Break(());
        }
    }
    if front {
        EDITING.with_borrow_mut(|e| e.insert((world, pos), (uuid, old)));
    }
    ControlFlow::Continue(())
}

/// HEAD of `lan_open_container` (cancellable): opening a chest, furnace... is that block's interact event,
/// decided now with a detached host ([Protection]).
pub(super) fn on_open_container(game: &mut super::Game<'_>, host: &mut super::lan::LanHost, conn: u32, pos: [i32; 3]) -> ControlFlow<bool> {
    if game.native_window() {
        return ControlFlow::Continue(());
    }
    let Some(g) = host.hub.guests.iter().find(|g| g.conn == conn) else { return ControlFlow::Continue(()) };
    let (uuid, world) = (g.uuid, g.dimension.clone());
    let state = state_string(game, pos);
    let allow = with_ess(game, |ess, h| {
        let mut detached = Detached { game: &mut *h.game };
        ess.on_interact(&mut detached, uuid, Some((&world, pos, &state)), None)
    });
    if allow == Some(Allow::No) { ControlFlow::Break(false) } else { ControlFlow::Continue(()) }
}

/// HEAD of `mark_block_entity_updated`: a sign whose editor was open and whose text changed was edited.
pub(super) fn on_block_entity_updated(game: &mut super::Game<'_>, key: (i32, i32, i32)) {
    if EDITING.with_borrow(HashMap::is_empty) {
        return;
    }
    let world = dim_id(game);
    let pos = [key.0, key.1, key.2];
    let Some((uuid, old)) = EDITING.with_borrow(|e| e.get(&(world.clone(), pos)).cloned()) else { return };
    if read_sign(game, &world, pos).is_some_and(|now| now != old) {
        EDITING.with_borrow_mut(|e| e.remove(&(world.clone(), pos)));
        EDITED.with_borrow_mut(|q| q.push((uuid, world, pos, old)));
    }
}

/// HEAD of `break_block_as_player` (cancellable): true cancels the break. Only a guest's breaks are asked.
pub(super) fn on_break(game: &mut super::Game<'_>, pos: [i32; 3]) -> bool {
    if game.native_window() {
        return false;
    }
    let Some(uuid) = game.lan_acting_uuid else { return false };
    let world = dim_id(game);
    let state = state_string(game, pos);
    let allow = with_ess(game, |ess, h| {
        let mut detached = Detached { game: &mut *h.game };
        ess.on_block_break(&mut detached, uuid, &world, pos, &state)
    });
    allow == Some(Allow::No)
}

/// HEAD of `native_tick`, after the core's: the player snapshot for detached hosts, their messages, and the
/// sign edits of the last tick.
pub(super) fn on_tick(game: &mut super::Game<'_>) {
    if game.native_window() {
        return;
    }
    let Some(hub) = game.lan.as_ref().map(|l| &l.hub) else { return };
    let players: Vec<(Uuid, String, u8, Loc)> = hub
        .guests
        .iter()
        .filter(|g| g.placed)
        .map(|g| (g.uuid, g.name.clone(), g.permission, Loc { world: g.dimension.clone(), x: g.position[0], y: g.position[1], z: g.position[2], yaw: g.yaw, pitch: g.pitch }))
        .collect();
    let online: Vec<Uuid> = players.iter().map(|p| p.0).collect();
    PLAYERS.with_borrow_mut(|p| *p = players);
    EDITING.with_borrow_mut(|e| e.retain(|_, (u, _)| online.contains(u)));
    let outbox = OUTBOX.with_borrow_mut(std::mem::take);
    if !outbox.is_empty() {
        let mut host = GameHost { game: &mut *game };
        for (to, text, bar) in outbox {
            match (to, bar) {
                (Sender::Player(p), true) => host.action_bar(p, &text),
                _ => host.send(to, &text),
            }
        }
    }
    for (uuid, world, pos) in CLICKED.with_borrow_mut(std::mem::take) {
        if super::Dim::from_id(&world) != game.dim {
            continue;
        }
        let state = state_string(game, pos);
        with_ess(game, |ess, h| {
            let slot = h.held_slot(uuid);
            let held = h.inventory(uuid).get(slot).cloned().flatten();
            ess.on_interact(h, uuid, Some((&world, pos, &state)), held.as_ref())
        });
    }
    for (uuid, world, pos, old) in EDITED.with_borrow_mut(std::mem::take) {
        let Some(now) = read_sign(game, &world, pos) else { continue };
        let mut lines = now.clone();
        let allow = with_ess(game, |ess, h| ess.on_sign_change(h, uuid, &world, pos, &mut lines));
        match allow {
            Some(Allow::No) => {
                write_sign(game, &world, pos, &old);
            }
            Some(Allow::Yes) if lines != now => {
                write_sign(game, &world, pos, &lines);
            }
            _ => {}
        }
    }
}

/// A Host for listeners that run while a guest's turn holds the hub (see the top of this file).
struct Detached<'a, 'g> {
    game: &'a mut super::Game<'g>,
}

fn snapshot<T>(player: Uuid, f: impl FnOnce(&(Uuid, String, u8, Loc)) -> T) -> Option<T> {
    PLAYERS.with_borrow(|p| p.iter().find(|e| e.0 == player).map(f))
}

impl Host for Detached<'_, '_> {
    fn now_millis(&mut self) -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
    }
    fn online(&mut self) -> Vec<Uuid> {
        PLAYERS.with_borrow(|p| p.iter().map(|e| e.0).collect())
    }
    fn name(&mut self, player: Uuid) -> Option<String> {
        snapshot(player, |e| e.1.clone())
    }
    fn lookup(&mut self, name: &str) -> Option<(Uuid, String)> {
        PLAYERS.with_borrow(|p| p.iter().find(|e| e.1.eq_ignore_ascii_case(name)).map(|e| (e.0, e.1.clone())))
    }
    fn op_level(&mut self, player: Uuid) -> u8 {
        snapshot(player, |e| e.2).unwrap_or(0)
    }
    fn send(&mut self, to: Sender, text: &[Span]) {
        OUTBOX.with_borrow_mut(|o| o.push((to, text.to_vec(), false)));
    }
    fn action_bar(&mut self, player: Uuid, text: &[Span]) {
        OUTBOX.with_borrow_mut(|o| o.push((Sender::Player(player), text.to_vec(), true)));
    }
    fn broadcast(&mut self, text: &[Span]) {
        for p in self.online() {
            self.send(Sender::Player(p), text);
        }
        self.send(Sender::Console, text);
    }
    fn location(&mut self, player: Uuid) -> Option<Loc> {
        snapshot(player, |e| e.3.clone())
    }
    fn block(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        (super::Dim::from_id(world) == self.game.dim).then(|| state_string(self.game, pos))
    }
    fn sign_lines(&mut self, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
        read_sign(self.game, world, pos)
    }
    fn set_sign_lines(&mut self, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
        write_sign(self.game, world, pos, lines)
    }
    fn item_is_block(&mut self, item: &str) -> bool {
        let id = if item.contains(':') { item.to_string() } else { format!("minecraft:{item}") };
        id != "minecraft:air" && self.game.pack.assets.registry.default_state(&id).is_some()
    }
}
