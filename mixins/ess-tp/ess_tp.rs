// horizon-essentials area `tp` (pure logic, standalone-tested): Teleport & places: tp*, tpa*, back, top, bottom, jump, tpr, settpr, home*, warp*, spawn, setspawn, world; warmups, cooldowns, safe teleports, /back on death.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    let _ = ess;
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
        perms(&mut t, "essentials.home, essentials.sethome", "");
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
        assert_eq!(t.ess.on_damage(&mut t.host, a, "entity_attack", 2.0), Allow::No);
        t.tick(81);
        assert!(!t.host.p(a).invulnerable);
        assert_eq!(t.ess.on_damage(&mut t.host, a, "entity_attack", 2.0), Allow::Yes);

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
        assert_eq!(t.said(b), ["Alice teleported you to Carol."]);
        assert_eq!(xyz(&mut t, b), ow(-20.5, 64.0, 7.5));
        assert_eq!(t.console("tp Carol Alice"), Vec::<String>::new());
        assert_eq!(t.said(c), ["Console teleported you to Alice."]);
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
        assert_eq!(t.cmd(a, "/setspawn VIP"), ["Spawn location set for group vip."]);
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
        let mut t = TestServer::new();
        let a = joined(&mut t, "Alice");
        t.op("Alice");
        at(&mut t, a, OW, 70.5, 64.0, 70.5);
        t.cmd(a, "/setspawn newbies");
        let b = t.join("Bob");
        t.tick(1);
        assert_eq!(xyz(&mut t, b), ow(70.5, 64.0, 70.5));
        t.tick(1);
        assert_eq!(t.said(a), ["Welcome Bob to the server!"]);
        assert_eq!(t.said(b), ["Welcome Bob to the server!"]);
        // Returning players are not announced or moved.
        t.quit(b);
        at(&mut t, b, OW, 3.5, 64.0, 3.5);
        t.join("Bob");
        t.tick(3);
        assert!(t.said(a).is_empty());
        assert_eq!(xyz(&mut t, b), ow(3.5, 64.0, 3.5));

        let mut t = TestServer::with_config("newbies:\n  spawnpoint: none\n  announce-format: ''");
        let a = t.join("Alice");
        at(&mut t, a, OW, 3.5, 64.0, 3.5);
        t.tick(3);
        assert!(t.said(a).is_empty());
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
