# Core requests from area `world`

What the world area needs from the core (lead-owned files) or from other areas, and what it does meanwhile.

1. **One owner for the block/interact injections.** AntiBuild (ess_protect) listens to `block_place`,
   `block_break` and `interact`, and Protect to `world`. No area emits `Ess::on_block_place`, `on_interact` or
   `on_world` from a Horizon hook yet (economy/items may hook `break_block_as_player` and `UseItemOn`). Please
   have one injection per event in the core bundle (or name the owning area), calling the core dispatcher, so
   every listener sees each event exactly once. Meanwhile AntiBuild only runs where something emits the event
   (/break emits `on_block_break` itself).
2. **Drop, pickup and craft events.** AntiBuild's drop/pickup/craft checks are `pub fn on_drop / on_pickup /
   on_craft` in ess_protect; the core has no such listener lists. Proposal: `listeners.drop`, `pickup`,
   `craft` (`fn(&mut Ess, &mut dyn Host, Uuid, &Item) -> Allow`).
3. **An attacker for damage.** The damage hook (`PlayerMotion::hurt_caused_by`) can't call Essentials, so the
   world glue mirrors Essentials' answers per vanilla damage type onto the player's body every second (see
   ess_glue_world.rs). PvP rules that depend on the attacker (god attacker without `essentials.god.pvp`,
   `protect.disable.pvp` with the attacker's `essentials.protect.pvp`) can't be decided there. A core
   `damage` event fired from Game-side code with the attacker (e.g. from `creatures.rs` attack paths) would fix it.
4. **A death event before drops, with Host access.** keepinv/keepxp run from a HEAD hook on
   `guest_forget_digging` (the first macro-free call in `guest_die`), where the hub is taken out of the game,
   so they use rules cached every second. A core `death_before_drops` event with a working Host would be exact.
5. **HTTP answers routing.** `Host::http_poll` drains every answer; the world area's tick listener routes them
   (GeoIP ids tag 0x4745, Discord 0x4449 in the high 16 bits). Other areas wanting HTTP should use their own
   tag and add a call in `ess_world::on_tick` (or the core could own the routing).
6. **GeoIP in /whois (admin area).** `crate::game::ess_geoip::whois_line(ess, host, player) -> Option<String>`
   returns the `whoisGeoLocation` line (MiniMessage); show it to the console and to viewers with
   `essentials.geoip.show`.
7. **Discord relays from other areas.** ess_discord exposes `relay_afk`, `relay_action` (/me), `relay_kick`,
   `relay_mute`, `relay_private` (msg), `relay_mail`, `relay_advancement` and `on_server_stop`; social/admin
   should call them where those events happen, and the core glue on shutdown.
8. **An observer chat event.** `Ess::on_chat` stops at the first non-Pass answer, so a relay (Discord, XMPP)
   inserted after a formatting listener never sees chat. ess_discord inserts its listener first as a
   workaround. Proposal: `listeners.chat_observed: Vec<fn(&mut Ess, &mut dyn Host, Uuid, &str)>` run before
   the deciding listeners.
9. **Host additions made in the world region** (please keep): `speeds`, `set_statistic`, `http_request`,
   `http_poll`, `tcp_open`, `tcp_send`, `tcp_close`, `tcp_poll`.
