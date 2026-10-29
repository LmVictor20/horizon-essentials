// horizon-essentials area `world` (pure logic, standalone-tested): Player, world & modules: fly, god, heal, feed, rest, speed, gamemode, ptime, pweather, suicide, ext, time, weather, thunder, tree, bigtree, break, spawnmob, spawner, remove, editsign, fun commands, Protect, AntiBuild, Discord, DiscordLink, XMPP, GeoIP.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    crate::game::ess_protect::register(ess);
    crate::game::ess_discord::register(ess);
    crate::game::ess_geoip::register(ess);
    crate::game::ess_xmpp::register(ess);
}
