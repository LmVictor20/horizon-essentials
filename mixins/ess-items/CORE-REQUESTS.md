# Core requests from the items area

1. **Ess access without a hub.** Packet handlers run while `game.lan` is taken (the hub is lent to
   `lan_guest_packet`), so `with_ess` gives a `GameHost` whose `hub()` is None: permission checks there see no
   name and op level 0. Items works around it (click policies computed per open on the tick, powertool/unlimited
   work queued to the scheduler). A core helper that lends the hub (`with_ess_hub(game, host: &mut LanHost, f)`)
   would let areas answer packet-time questions directly.
2. **Block place event.** Nothing calls `Ess::on_block_place`. Items hooks `under_spawn_protection` (it runs
   before every UseItemOn) only for its own unlimited-items snapshot and does not call `on_block_place`, to not
   collide with an AntiBuild hook. A core-owned place hook (with the placed state) would serve both.
3. **Host region additions (items, done in the items regions):** `recipes`, `can_enchant`, `closed_containers`,
   `opened_containers`, `set_container_policy`, `item_uses`. Other areas opening containers
   (`Host::open_container`, admin's /invsee) get clicks through `container_click` (asked once per raw slot right
   after the open: answers must not depend on the click's moment) and closes through `container_close`.
4. **Handbook correction:** `lan_guest_packet` cannot take a HEAD hook (it contains macros: `matches!`);
   `lan_table_valid` cannot take RETURN (a call under a reference). Items uses `lan_merchant_click` (first guard of
   every ContainerClick), `guest_attack`, `hand::reset_attack_strength` (Punch) and `under_spawn_protection`.
