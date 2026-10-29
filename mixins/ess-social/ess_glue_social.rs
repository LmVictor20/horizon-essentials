// horizon-essentials glue for area `social` (game only): `crate::game::ess_glue_social`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here), and the tick that
// serves this area's hub hooks. The hub (horizon_client library) and this glue (game binary) share the per-hub
// state declared in horizon-mixin.toml: `.0` JSON (on, join, quit, bypass, display, tab, seen, tree, orig) and `.1`
// the chat messages held for Essentials.

use super::ess_glue::{GameHost, component, nbt, with_ess};
use crate::game::ess_core::*;
use horizon_protocol::packets::play::{self, Clientbound as C};
use serde_json::{Value, json};

fn key(u: Uuid) -> String {
    format!("{u:032x}")
}

/// The JSON half of the shared hub state, made an object.
fn shared(hub: &mut horizon_client::lan::hub::Hub) -> &mut Value {
    let state = &mut hub.__horizon_mixins_horizon_essentials_social_ess_mut().0;
    if !state.is_object() {
        *state = json!({});
    }
    state
}

fn put(map: &mut Value, field: &str, player: Uuid, value: Option<Value>) {
    if !map[field].is_object() {
        map[field] = json!({});
    }
    let k = key(player);
    match value {
        Some(v) => map[field][k] = v,
        None => {
            if let Some(m) = map[field].as_object_mut() {
                m.remove(&k);
            }
        }
    }
}

/// Every tick after the core's: the chat held by the hub hook goes through Essentials, the command tree gets
/// Essentials' labels once, and the join/quit flags follow the config.
pub(super) fn on_tick(game: &mut super::Game<'_>) {
    let _ = with_ess(game, |ess, host| host.social_tick(ess));
}

/// A player's /say, /me or /msg (tell, w) that vanilla would run itself: queued for Essentials when it claims the
/// label (through the core's command hook, as that player). True when claimed.
pub(super) fn claim_message_command(game: &mut super::Game<'_>, player: Option<Uuid>, command: &str) -> bool {
    let label = command.trim_start_matches('/').split(' ').next().unwrap_or("").to_lowercase();
    if !matches!(label.as_str(), "say" | "me" | "msg" | "tell" | "w") || player.is_none() {
        return false;
    }
    if with_ess(game, |ess, _| ess.claims(command)) != Some(true) {
        return false;
    }
    let previous = std::mem::replace(&mut game.lan_acting_uuid, player);
    let mut ran = false;
    super::ess_glue::on_command(game, command, &mut ran);
    game.lan_acting_uuid = previous;
    ran
}

impl GameHost<'_, '_> {
    pub(super) fn social_broadcast(&mut self, text: &[Span]) {
        for p in self.online() {
            self.send(Sender::Player(p), text);
        }
        self.send(Sender::Console, text);
    }
    pub(super) fn social_chat_as(&mut self, _player: Uuid, text: &[Span]) {
        self.social_broadcast(text);
    }
    /// The chat sender name (`Hub::player_display_name` hook): the spans of the display name.
    pub(super) fn social_set_display_name(&mut self, player: Uuid, name: Option<&[Span]>) {
        let Some(hub) = self.hub_mut() else { return };
        let extra = name.map(|spans| component(spans).get("extra").cloned().unwrap_or_else(|| json!([])));
        put(shared(hub), "display", player, extra);
    }
    /// The tab list name: kept for joiners (`Hub::player_info` hook) and sent to everyone now.
    pub(super) fn social_set_tab_name(&mut self, player: Uuid, name: Option<&[Span]>) {
        let Some(hub) = self.hub_mut() else { return };
        put(shared(hub), "tab", player, name.map(component));
        let entry = play::PlayerInfo { uuid: player, display_name: Some(name.map(nbt)), ..Default::default() };
        hub.broadcast(&C::PlayerInfoUpdate(play::PlayerInfoUpdate { actions: 0x20, entries: vec![entry] }));
    }
    /// No Essentials feature changes the server list MOTD; the status line keeps server.properties' motd.
    pub(super) fn social_set_motd(&mut self, _motd: &[Span]) {}

    fn social_tick(&mut self, ess: &mut Ess) {
        let join = crate::game::ess_social::join_messages_replaced(ess);
        let quit = crate::game::ess_social::quit_messages_replaced(ess);
        let Some(hub) = self.hub_mut() else { return };
        let state = shared(hub);
        state["on"] = json!(true);
        state["join"] = json!(join);
        state["quit"] = json!(quit);
        if !state["tree"].is_object() {
            state["tree"] = crate::game::ess_social::command_tree(ess);
            let orig = state["orig"].clone();
            let key = hub.mod_commands_key().unwrap_or(u64::MAX);
            hub.set_mod_commands(key, orig);
        }
        // Players who joined got the tab list without Essentials' names: send them again when the list changes.
        let online: Vec<Value> = hub.guests.iter().filter(|g| g.placed).map(|g| json!(key(g.uuid))).collect();
        let state = shared(hub);
        if state["seen"] != json!(online) {
            state["seen"] = json!(online);
            let entries: Vec<play::PlayerInfo> = state["tab"]
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| {
                            let uuid = u128::from_str_radix(k, 16).ok()?;
                            Some(play::PlayerInfo { uuid, display_name: Some(Some(horizon_client::lan::data::component_nbt(v))), ..Default::default() })
                        })
                        .collect()
                })
                .unwrap_or_default();
            if !entries.is_empty() {
                hub.broadcast(&C::PlayerInfoUpdate(play::PlayerInfoUpdate { actions: 0x20, entries }));
            }
        }
        let pending = std::mem::take(&mut hub.__horizon_mixins_horizon_essentials_social_ess_mut().1);
        for (message, bound, line) in pending {
            match ess.on_chat(self, message.sender, &message.content) {
                Chat::Pass => {
                    let mut events = Vec::new();
                    if let Some(hub) = self.hub_mut() {
                        shared(hub)["bypass"] = json!(true);
                        hub.broadcast_chat(&message, &bound, line, &mut events);
                        shared(hub)["bypass"] = json!(false);
                    }
                    for event in events {
                        self.game.lan_log_event(event);
                    }
                }
                Chat::Cancel => {}
                Chat::Replace(lines) => {
                    for (i, (to, spans)) in lines.into_iter().enumerate() {
                        match to {
                            None => self.social_broadcast(&spans),
                            Some(list) => {
                                for p in list {
                                    self.send(Sender::Player(p), &spans);
                                }
                                if i == 0 {
                                    self.send(Sender::Console, &spans);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
