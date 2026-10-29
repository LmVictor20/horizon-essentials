// horizon-essentials area `world` (pure logic, standalone-tested): Player, world & modules: fly, god, heal, feed, rest, speed, gamemode, ptime, pweather, suicide, ext, time, weather, thunder, tree, bigtree, break, spawnmob, spawner, remove, editsign, fun commands, Protect, AntiBuild, Discord, DiscordLink, XMPP, GeoIP.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    crate::game::ess_protect::register(ess);
    crate::game::ess_discord::register(ess);
    crate::game::ess_geoip::register(ess);
    crate::game::ess_xmpp::register(ess);
}

/// What happens to a dying player's things (keepinv, keepxp, curse policies).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeathPolicy {
    pub keep_inventory: bool,
    pub keep_level: bool,
    /// Items to drop although the inventory is kept (curses with policy `drop`).
    pub drop: Vec<Item>,
}

pub fn death_policy(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> DeathPolicy {
    let _ = (ess, host, player);
    DeathPolicy::default()
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
        assert_eq!(t.cmd(alice, "/ptime reset *"), ["Player time has been reset for: Alice, Bobby."]);
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
        assert_eq!(t.cmd(alice, "/pweather reset *"), ["Player weather has been reset for: Alice, Bobby."]);
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
