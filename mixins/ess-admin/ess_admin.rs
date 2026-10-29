// horizon-essentials area `admin` (pure logic, standalone-tested): Moderation & admin: ban*, tempban*, unban*, kick*, jail*, sudo, kill, seen, whois, vanish, invsee, gc, essentials, backup, list, near, getpos.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    let _ = ess;
}

pub fn parse_date_diff(_time: &str, _future: bool, _empty_epoch: bool, _now: i64) -> Option<i64> {
    None
}
pub fn remove_time_pattern(input: &str) -> String {
    input.to_string()
}
pub fn format_date_diff(_ess: &Ess, _now: i64, _date: i64) -> String {
    String::new()
}
pub fn cooldown_matches(_pattern: &str, _text: &str) -> bool {
    false
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ess_testkit::*;
    use serde_json::json;

    const NOW: i64 = 1_700_000_000_000;
    const HOUR: i64 = 3_600_000;
    const DAY: i64 = 24 * HOUR;
    const PLAY: &str = "minecraft:custom/minecraft:play_time";

    fn server() -> (TestServer, Uuid, Uuid) {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        t.op("Alice");
        let bob = t.join("Bob");
        t.host.said(Sender::Console);
        (t, alice, bob)
    }

    fn ip(t: &mut TestServer, p: Uuid) -> String {
        t.host.p(p).address.clone()
    }

    fn logs(t: &mut TestServer) -> Vec<String> {
        std::mem::take(&mut t.host.logs)
    }

    fn none() -> Vec<String> {
        Vec::new()
    }

    // ----- DateUtil ---------------------------------------------------------------------------------

    #[test]
    fn date_diffs_parse_like_essentials() {
        assert_eq!(parse_date_diff("1d2h", true, false, NOW), Some(NOW + 26 * HOUR));
        assert_eq!(parse_date_diff("1d 2h", true, false, NOW), Some(NOW + 26 * HOUR));
        assert_eq!(parse_date_diff("30m", true, false, NOW), Some(NOW + 30 * 60_000));
        assert_eq!(parse_date_diff("2w", true, false, NOW), Some(NOW + 14 * DAY));
        assert_eq!(parse_date_diff("45", true, false, NOW), Some(NOW + 45_000));
        assert_eq!(parse_date_diff("5seconds", true, false, NOW), Some(NOW + 5_000));
        assert_eq!(parse_date_diff("1h", false, false, NOW), Some(NOW - HOUR));
        assert_eq!(parse_date_diff("10m", true, true, NOW), Some(600_000));
        // 2023-11-14 + 1 month = 2023-12-14 (30 days later).
        assert_eq!(parse_date_diff("1mo", true, false, NOW), Some(NOW + 30 * DAY));
        // Capped at ten years from now.
        let ten = parse_date_diff("10y", true, false, NOW).unwrap();
        assert_eq!(parse_date_diff("500y", true, false, NOW), Some(ten));
        assert_eq!(parse_date_diff("abc", true, false, NOW), None);
        assert_eq!(parse_date_diff("", true, false, NOW), None);
        // The first non-empty match counts, even after words.
        assert_eq!(parse_date_diff("spam 1d", true, false, NOW), Some(NOW + DAY));
    }

    #[test]
    fn time_patterns_are_removed_from_the_start_only() {
        assert_eq!(remove_time_pattern("1d2h Griefing again"), "Griefing again");
        assert_eq!(remove_time_pattern("1d"), "");
        assert_eq!(remove_time_pattern("Griefing 1d"), "Griefing 1d");
    }

    #[test]
    fn date_diffs_format_like_essentials() {
        let t = TestServer::new();
        let f = |d: i64| format_date_diff(&t.ess, NOW, d);
        assert_eq!(f(NOW), "now");
        assert_eq!(f(NOW + DAY), "1 day");
        assert_eq!(f(NOW + 2 * DAY), "2 days");
        assert_eq!(f(NOW + DAY + HOUR + 60_000 + 1_000), "1 day 1 hour 1 minute");
        assert_eq!(f(NOW - 2 * HOUR), "2 hours");
        assert_eq!(f(NOW + 32 * DAY), "1 month 2 days");
        assert_eq!(f(NOW + 366 * DAY), "1 year 1 day");
        assert_eq!(f(NOW + 10), "now");
        assert_eq!(f(NOW + 61_000), "1 minute 1 second");
    }

    // ----- Permissions provider ----------------------------------------------------------------------

    #[test]
    fn permissions_groups_inheritance_wildcards_and_negation() {
        let mut t = TestServer::new();
        t.permissions(
            "groups:\n  Default:\n    default: true\n    prefix: '&7'\n    permissions: [essentials.seen, essentials.list]\n  mod:\n    inherits: [default]\n    prefix: '&9[Mod] '\n    suffix: '!'\n    permissions: ['essentials.kick', 'essentials.jail.*', -essentials.list]\n  admin:\n    inherits: [mod]\n    permissions: ['*', -essentials.ban.exempt]\n  loop1:\n    inherits: [loop2]\n    permissions: [a.b]\n  loop2:\n    inherits: [loop1]\nusers:\n  carol:\n    groups: [mod]\n    permissions: [essentials.fly]\n  DAVE:\n    groups: [admin]\n  erin:\n    groups: [loop1]\n",
        );
        let bob = t.join("Bob");
        let carol = t.join("Carol");
        let dave = t.join("Dave");
        let erin = t.join("Erin");
        let has = |t: &mut TestServer, p: Uuid, n: &str| t.ess.has(&mut t.host, Sender::Player(p), n);
        // Default group (marked default; names are case-insensitive).
        assert!(has(&mut t, bob, "essentials.seen"));
        assert!(has(&mut t, bob, "ESSENTIALS.LIST"));
        assert!(!has(&mut t, bob, "essentials.kick"));
        // Inheritance and the group's own negation of an inherited node.
        assert!(has(&mut t, carol, "essentials.seen"));
        assert!(has(&mut t, carol, "essentials.kick"));
        assert!(!has(&mut t, carol, "essentials.list"));
        // Branch wildcards.
        assert!(has(&mut t, carol, "essentials.jail.allow-break"));
        assert!(has(&mut t, carol, "essentials.jail.allow.home"));
        assert!(!has(&mut t, carol, "essentials.jailx"));
        // User nodes.
        assert!(has(&mut t, carol, "essentials.fly"));
        // `*` with a negated node, multi-level inheritance.
        assert!(has(&mut t, dave, "essentials.anything.at.all"));
        assert!(!has(&mut t, dave, "essentials.ban.exempt"));
        assert!(has(&mut t, dave, "essentials.kick"));
        // An inheritance loop terminates.
        assert!(has(&mut t, erin, "a.b"));
        assert!(!has(&mut t, erin, "essentials.kick"));
        // Groups, primary first; prefixes loaded.
        assert_eq!(t.ess.group(&mut t.host, carol), "mod");
        assert_eq!(t.ess.group(&mut t.host, bob), "default");
        assert_eq!(t.ess.perms.groups["mod"].prefix, "&9[Mod] ");
        assert_eq!(t.ess.perms.groups["mod"].suffix, "!");
    }

    #[test]
    fn permissions_fall_back_to_plugin_yml_defaults_by_op() {
        let mut t = TestServer::new();
        t.permissions("groups: {}\n");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        t.op("Alice");
        let has = |t: &mut TestServer, p: Uuid, n: &str| t.ess.has(&mut t.host, Sender::Player(p), n);
        assert!(has(&mut t, alice, "essentials.ban"));
        assert!(!has(&mut t, bob, "essentials.ban"));
        // default: false nodes are not given to ops.
        assert!(!has(&mut t, alice, "essentials.ban.exempt"));
        assert!(!has(&mut t, alice, "essentials.kick.exempt"));
        // Unknown nodes follow op status; the console has everything.
        assert!(has(&mut t, alice, "essentials.jail.allow.home"));
        assert!(!has(&mut t, bob, "essentials.jail.allow.home"));
        assert!(t.ess.has(&mut t.host, Sender::Console, "essentials.ban.exempt"));
    }

    #[test]
    fn permissions_users_by_uuid_and_essentials_reload() {
        let mut t = TestServer::new();
        let bob = t.join("Bob");
        assert!(!t.ess.has(&mut t.host, Sender::Player(bob), "essentials.kick"));
        std::fs::write(
            t.ess.dir.join("permissions.yml"),
            format!("groups:\n  default:\n    default: true\nusers:\n  '{}':\n    permissions: [essentials.kick]\n", uuid_string(bob)),
        )
        .unwrap();
        assert_eq!(t.console("essentials reload"), ["Essentials reloaded 2.21.0-horizon."]);
        assert!(t.ess.has(&mut t.host, Sender::Player(bob), "essentials.kick"));
    }

    // ----- /ban /tempban /banip /tempbanip /unban /unbanip, login enforcement -------------------------

    #[test]
    fn ban_kicks_logs_and_notifies() {
        let (mut t, alice, bob) = server();
        let carol = t.join("Carol");
        assert_eq!(t.cmd(alice, "/ban Bob Griefing &cbadly"), ["Player Alice banned Bob for: Griefing badly."]);
        assert_eq!(t.host.kicked, [(bob, "You have been banned:\nGriefing badly".to_string())]);
        assert_eq!(t.host.bans.len(), 1);
        assert_eq!(t.host.bans[0].target, "Bob");
        assert_eq!(t.host.bans[0].reason, "Griefing §cbadly");
        assert_eq!(t.host.bans[0].source, "Alice");
        assert_eq!(t.host.bans[0].expires, None);
        assert_eq!(logs(&mut t), ["Player Alice banned Bob for: You have been banned:", "Griefing badly."]);
        // Carol has no essentials.ban.notify.
        assert!(t.said(carol).is_empty());
    }

    #[test]
    fn ban_default_reason_unknown_users_and_usage() {
        let (mut t, _alice, _bob) = server();
        assert_eq!(t.console("ban Zed"), ["Warning: The user 'Zed' has never joined this server."]);
        assert_eq!(t.host.bans[0].reason, "The Ban Hammer has spoken!");
        assert_eq!(t.host.bans[0].source, "Console");
        assert_eq!(
            t.console("ban"),
            ["Command Help: /ban", "Description: Bans a player.", "Usage(s);", "/ban <player> [reason] - Bans the specified player with an optional reason"]
        );
        assert_eq!(t.console("eban Bob x|y"), none());
        assert_eq!(t.host.bans.iter().find(|b| b.target == "Bob").unwrap().reason, "x\ny");
    }

    #[test]
    fn ban_exempt_and_offline_nodes() {
        let (mut t, alice, _bob) = server();
        t.permissions("groups:\n  default:\n    default: true\n  mod:\n    permissions: [essentials.ban]\nusers:\n  bob:\n    permissions: [essentials.ban.exempt]\n  carol:\n    groups: [mod]\n");
        let carol = t.join("Carol");
        assert_eq!(t.cmd(alice, "/ban Bob"), ["Error: You cannot ban that player."]);
        assert!(t.host.bans.is_empty());
        // The console ignores exemption.
        t.console("ban Bob");
        assert_eq!(t.host.bans.len(), 1);
        t.host.bans.clear();
        let dave = t.join("Dave");
        t.quit(dave);
        assert_eq!(t.cmd(carol, "/ban Dave"), ["Error: You may not ban offline players."]);
        assert_eq!(t.cmd(alice, "/ban Dave"), ["Player Alice banned Dave for: The Ban Hammer has spoken!."]);
    }

    #[test]
    fn banned_players_are_refused_at_join_with_essentials_messages() {
        let (mut t, _alice, bob) = server();
        t.console("ban Bob Cheating");
        t.join("Bob");
        assert_eq!(t.host.kicked.last().unwrap(), &(bob, "You are banned from this server. Reason: Cheating".to_string()));
        assert!(!t.host.p(bob).online);
    }

    #[test]
    fn tempban_kicks_and_expires() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/tempban Bob 1d2h Spamming chat"), ["Player Alice temporarily banned Bob for 1 day 2 hours: Spamming chat."]);
        assert_eq!(t.host.kicked, [(bob, "You have been temporarily banned for 1 day 2 hours:\nSpamming chat".to_string())]);
        assert_eq!(t.host.bans[0].expires, Some(NOW + 26 * HOUR));
        assert_eq!(logs(&mut t), ["Player Alice temporarily banned Bob for 1 day 2 hours: Spamming chat."]);
        t.host.now += 2 * HOUR;
        t.join("Bob");
        assert_eq!(t.host.kicked.last().unwrap().1, "You are banned from this server for 1 day. Reason: Spamming chat");
        // After expiry the player gets in and the entry is gone.
        t.host.now += DAY;
        let kicks = t.host.kicked.len();
        t.join("Bob");
        assert_eq!(t.host.kicked.len(), kicks);
        assert!(t.host.p(bob).online);
        assert!(t.host.bans.is_empty());
    }

    #[test]
    fn tempban_default_reason_limits_and_errors() {
        let (mut t, _alice, _bob) = server();
        assert_eq!(t.console("tempban Bob 1h"), none());
        assert_eq!(t.host.bans[0].reason, "The Ban Hammer has spoken!");
        assert_eq!(t.host.kicked[0].1, "You have been temporarily banned for 1 hour:\nThe Ban Hammer has spoken!");
        assert_eq!(
            t.console("tempban Bob"),
            [
                "Command Help: /tempban",
                "Description: Temporary ban a user.",
                "Usage(s);",
                "/tempban <player> <datediff> [reason] - Bans the given player for the specified amount of time with an optional reason"
            ]
        );
        assert_eq!(t.console("tempban Bob forever"), ["Error: Illegal date format."]);
        let mut t = TestServer::with_config("max-tempban-time: 3600");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.tempban, essentials.tempban.offline]\nusers:\n  bob:\n    permissions: [essentials.tempban.exempt]\n");
        let carol = t.join("Carol");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(carol, "/tempban Bob 1h"), ["You may not tempban that player."]);
        t.quit(bob);
        assert_eq!(t.cmd(carol, "/tempban Bob 2h"), ["You may not ban a player for this period of time."]);
        assert_eq!(t.cmd(carol, "/tempban Bob 30m"), none());
        assert_eq!(t.host.bans.len(), 1);
    }

    #[test]
    fn tempban_offline_needs_the_node() {
        let mut t = TestServer::new();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.tempban]\n");
        let carol = t.join("Carol");
        let bob = t.join("Bob");
        t.quit(bob);
        assert_eq!(t.cmd(carol, "/tempban Bob 1h"), ["You may not tempban offline players."]);
    }

    #[test]
    fn banip_by_address_and_by_player() {
        let (mut t, alice, bob) = server();
        let bob_ip = ip(&mut t, bob);
        assert_eq!(t.cmd(alice, "/banip Bob Proxy"), [format!("Player Alice banned IP address {bob_ip} for: Proxy.")]);
        assert_eq!(t.host.ip_bans[0].target, bob_ip);
        assert_eq!(t.host.kicked, [(bob, "You have been banned:\nProxy".to_string())]);
        assert_eq!(logs(&mut t), [format!("Player Alice banned IP address {bob_ip} for: Proxy.")]);
        t.join("Bob");
        assert_eq!(t.host.kicked.last().unwrap().1, "Your IP address is banned from this server. Reason: Proxy");
        t.console("banip 10.0.0.1");
        assert_eq!(t.host.ip_bans[1].target, "10.0.0.1");
        assert_eq!(t.host.ip_bans[1].reason, "The Ban Hammer has spoken!");
        logs(&mut t);
        assert_eq!(t.console("unbanip 10.0.0.1"), none());
        assert_eq!(logs(&mut t).last().unwrap(), "Player Console unbanned IP: 10.0.0.1");
        assert_eq!(t.cmd(alice, "/unbanip Bob"), [format!("Player Alice unbanned IP: {bob_ip}")]);
        assert!(t.host.ip_bans.is_empty());
    }

    #[test]
    fn tempbanip_keeps_essentials_argument_order() {
        let (mut t, alice, bob) = server();
        let bob_ip = ip(&mut t, bob);
        assert_eq!(t.cmd(alice, "/tempbanip Bob 1h Spam"), [format!("Player Alice temporarily banned IP address {bob_ip} for Spam: 1 hour.")]);
        assert_eq!(t.host.ip_bans[0].expires, Some(NOW + HOUR));
        assert_eq!(t.host.kicked[0].1, "You have been banned:\nSpam");
        t.join("Bob");
        assert_eq!(t.host.kicked.last().unwrap().1, "Your IP address is temporarily banned from this server for 1 hour. Reason: Spam");
    }

    #[test]
    fn unban_known_and_unknown_players() {
        let (mut t, alice, _bob) = server();
        t.console("ban Bob");
        logs(&mut t);
        assert_eq!(t.cmd(alice, "/unban Bob"), ["Player Alice unbanned Bob"]);
        assert!(t.host.bans.is_empty());
        assert_eq!(logs(&mut t), ["Player Alice unbanned Bob"]);
        assert_eq!(t.console("pardon Zed"), ["Error: Player Zed was never on this server."]);
        t.host.bans.push(Ban { target: "Zed".into(), reason: "x".into(), source: "Server".into(), expires: None });
        t.console("unban Zed");
        assert!(t.host.bans.is_empty());
    }

    // ----- /kick /kickall --------------------------------------------------------------------------------

    #[test]
    fn kick_with_reason_default_and_notify() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/kick Bob Go &lhome|now"), ["Player Alice kicked Bob for Go home", "now."]);
        assert_eq!(t.host.kicked, [(bob, "Go home\nnow".to_string())]);
        t.join("Bob");
        assert_eq!(t.console("kick Bob"), none());
        assert_eq!(t.host.kicked[1].1, "Kicked from server.");
        assert_eq!(logs(&mut t).last().unwrap(), "Player Console kicked Bob for Kicked from server..");
        assert_eq!(t.said(alice).last().unwrap(), "Player Console kicked Bob for Kicked from server..");
        assert_eq!(t.console("kick Bob"), ["Error: Player not found."]);
    }

    #[test]
    fn kick_exempt_and_hidden_targets() {
        let (mut t, alice, _bob) = server();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.kick]\nusers:\n  bob:\n    permissions: [essentials.kick.exempt]\n");
        let carol = t.join("Carol");
        assert_eq!(t.cmd(carol, "/kick Bob"), ["Error: You cannot kick that person."]);
        t.console("kick Bob");
        assert_eq!(t.host.kicked.len(), 1);
        t.cmd(alice, "/vanish");
        assert_eq!(t.cmd(carol, "/kick Alice"), ["Error: Player not found."]);
        assert_eq!(t.host.kicked.len(), 1);
    }

    #[test]
    fn kickall_spares_the_sender_and_exempt_players() {
        let (mut t, alice, bob) = server();
        let carol = t.join("Carol");
        let dave = t.join("Dave");
        t.op("Dave");
        assert_eq!(t.cmd(alice, "/kickall Restarting"), ["Kicked all players from server."]);
        let kicked: Vec<Uuid> = t.host.kicked.iter().map(|k| k.0).collect();
        assert_eq!(kicked.len(), 2);
        assert!(kicked.contains(&bob) && kicked.contains(&carol));
        assert!(t.host.p(dave).online && t.host.p(alice).online);
        assert_eq!(t.host.kicked[0].1, "Restarting");
    }

    // ----- /kill /sudo --------------------------------------------------------------------------------------

    #[test]
    fn kill_players_exempt_force_and_cancelled_damage() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/kill Bob"), ["Killed Bob."]);
        assert_eq!(t.host.p(bob).health, 0.0);
        t.host.p(bob).health = 20.0;
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.kill]\n");
        let carol = t.join("Carol");
        assert_eq!(t.cmd(carol, "/kill Alice"), ["Error: You cannot kill Alice."]);
        assert_eq!(t.cmd(carol, "/kill b"), ["Error: Player not found."]);
        t.ess.listeners.damage.push(|_, _, _, _, _| Allow::No);
        assert_eq!(t.cmd(carol, "/kill Bob"), none());
        assert_eq!(t.host.p(bob).health, 20.0);
        // kill.force ignores a cancelled damage event.
        assert_eq!(t.cmd(alice, "/kill Bob"), ["Killed Bob."]);
        t.host.p(bob).health = 20.0;
        assert_eq!(t.console("kill *").len(), 3);
        assert_eq!(t.host.p(carol).health, 0.0);
        assert_eq!(t.console("kill"), ["Command Help: /kill", "Description: Kills specified player.", "Usage(s);", "/kill <player> - Kills the specified player"]);
    }

    #[test]
    fn sudo_runs_commands_and_chat_next_tick() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.console("sudo Bob ping"), ["Forcing Bob to run: /ping"]);
        assert!(t.said(bob).is_empty());
        t.tick(1);
        assert_eq!(t.said(bob), ["Pong!"]);
        t.console("sudo Bob time set day");
        t.tick(1);
        assert_eq!(t.host.player_commands, [(bob, "time set day".to_string(), false)]);
        t.console("sudo Bob c:hello there");
        assert_eq!(t.host.chats.last().unwrap(), &(bob, "<Bob> hello there".to_string()));
        // Self is silently skipped.
        assert_eq!(t.cmd(alice, "/sudo Alice ping"), ["Forcing Alice to run: /ping"]);
        t.tick(1);
        assert!(t.said(alice).is_empty());
        assert_eq!(
            t.cmd(alice, "/sudo Bob"),
            ["Command Help: /sudo", "Description: Make another user perform a command.", "Usage(s);", "/sudo <player> <command> [args] - Makes the specified player run the given command"]
        );
    }

    #[test]
    fn sudo_exempt_only_for_players() {
        let (mut t, alice, bob) = server();
        t.permissions("groups:\n  default:\n    default: true\nusers:\n  bob:\n    permissions: [essentials.sudo.exempt]\n");
        assert_eq!(t.cmd(alice, "/sudo Bob ping"), ["Forcing Bob to run: /ping", "You cannot sudo Bob."]);
        t.tick(1);
        assert!(t.said(bob).is_empty());
        t.console("sudo Bob ping");
        t.tick(1);
        assert_eq!(t.said(bob), ["Pong!"]);
        t.console("sudo * ping");
        t.tick(1);
        assert_eq!(t.said(alice), ["Pong!"]);
    }

    // ----- /vanish /list /near -------------------------------------------------------------------------------

    #[test]
    fn vanish_toggles_hides_and_reports() {
        let (mut t, alice, bob) = server();
        assert_eq!(t.cmd(alice, "/vanish"), ["Vanish for Alice: enabled", "You are now completely invisible to normal users, and hidden from in-game commands."]);
        assert!(t.host.p(alice).hidden);
        assert!(t.ess.is_vanished(alice));
        assert!(!t.ess.can_see(&mut t.host, Sender::Player(bob), alice));
        assert!(t.host.console_commands.contains(&"effect give Alice minecraft:invisibility infinite 1 false".to_string()));
        assert_eq!(t.cmd(alice, "/v off"), ["Vanish for Alice: disabled"]);
        assert!(!t.host.p(alice).hidden);
        assert!(t.host.console_commands.contains(&"effect clear Alice minecraft:invisibility".to_string()));
        assert_eq!(t.cmd(alice, "/vanish Bob"), ["Vanish for Bob: enabled"]);
        assert_eq!(t.said(bob), ["Vanish for Bob: enabled", "You are now completely invisible to normal users, and hidden from in-game commands."]);
        assert_eq!(t.console("vanish Bob off"), ["Vanish for Bob: disabled"]);
        // Leaving makes them visible again.
        t.cmd(alice, "/vanish on");
        t.quit(alice);
        assert!(!t.ess.is_vanished(alice));
    }

    #[test]
    fn vanish_without_others_node_toggles_self() {
        let mut t = TestServer::new();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.vanish]\n");
        let bob = t.join("Bob");
        t.join("Carol");
        assert_eq!(t.cmd(bob, "/vanish Carol")[0], "Vanish for Bob: enabled");
    }

    #[test]
    fn list_counts_and_groups_hidden_players() {
        let (mut t, alice, bob) = server();
        let carol = t.join("Carol");
        assert_eq!(t.cmd(bob, "/list"), ["There are 3 out of maximum 20 players online.", "default: Alice, Bob, Carol"]);
        t.cmd(alice, "/vanish");
        assert_eq!(t.cmd(bob, "/who"), ["There are 2 out of maximum 20 players online.", "default: Bob, Carol"]);
        assert_eq!(t.cmd(alice, "/list"), ["There are 2/1 out of maximum 20 players online.", "default: [HIDDEN]Alice, Bob, Carol"]);
        assert_eq!(t.console("list"), ["There are 2/1 out of maximum 20 players online.", "default: [HIDDEN]Alice, Bob, Carol"]);
        assert_eq!(t.cmd(carol, "/list default"), ["There are 2 out of maximum 20 players online.", "Default: Bob, Carol"]);
        assert_eq!(t.cmd(carol, "/list nope"), ["There are 2 out of maximum 20 players online.", "Error: There's no one online in this group!"]);
    }

    #[test]
    fn list_config_merges_hides_and_truncates_groups() {
        let mut t = TestServer::with_config("list:\n  Staff: mod admin\n");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.list]\n  mod: {}\n  admin: {}\nusers:\n  alice:\n    groups: [admin]\n  bob:\n    groups: [mod]\n");
        t.join("Alice");
        t.join("Bob");
        let carol = t.join("Carol");
        assert_eq!(t.cmd(carol, "/list"), ["There are 3 out of maximum 20 players online.", "Staff: Alice, Bob", "default: Carol"]);
        let mut t = TestServer::with_config("list:\n  default: hidden\n");
        let carol = t.join("Carol");
        assert_eq!(t.cmd(carol, "/list"), ["There are 1 out of maximum 20 players online."]);
        let mut t = TestServer::with_config("list:\n  default: 1\n");
        let carol = t.join("Carol");
        t.join("Dave");
        assert_eq!(t.cmd(carol, "/list"), ["There are 2 out of maximum 20 players online.", "default: 2 online, for the full list: /list default"]);
        let mut t = TestServer::with_config("list:\n  Players: '*'\n");
        let carol = t.join("Carol");
        assert_eq!(t.cmd(carol, "/list"), ["There are 1 out of maximum 20 players online.", "Players: Carol"]);
    }

    #[test]
    fn near_lists_players_in_radius() {
        let (mut t, alice, bob) = server();
        let carol = t.join("Carol");
        t.host.p(bob).loc = Loc::new("minecraft:overworld", 10.5, 64.0, 0.5);
        t.host.p(carol).loc = Loc::new("minecraft:overworld", 3.5, 64.0, 4.5);
        assert_eq!(t.cmd(alice, "/near"), ["Players nearby: Carol(5m), Bob(10m)"]);
        assert_eq!(t.cmd(alice, "/near 7"), ["Players nearby: Carol(5m)"]);
        assert_eq!(t.cmd(alice, "/near Bob 8"), ["Players nearby: Carol(8m)"]);
        assert_eq!(t.cmd(bob, "/near 500"), ["Radius is too big! Maximum radius is 200.", "Players nearby: Carol(8m), Alice(10m)"]);
        t.host.p(carol).loc = Loc::new("minecraft:the_nether", 3.5, 64.0, 4.5);
        t.cmd(alice, "/vanish");
        assert_eq!(t.cmd(bob, "/near"), ["Players nearby: none"]);
        assert_eq!(t.console("near Alice"), ["Players nearby: Bob(10m)"]);
    }

    // ----- /getpos /compass /depth /gc /playtime ---------------------------------------------------------------

    #[test]
    fn getpos_self_others_and_console() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).loc.yaw = -90.0;
        t.host.p(alice).loc.pitch = 12.5;
        assert_eq!(
            t.cmd(alice, "/getpos"),
            ["Current World: world", "X: 0 (+East <-> -West)", "Y: 64 (+Up <-> -Down)", "Z: 0 (+South <-> -North)", "Yaw: 270 (Rotation)", "Pitch: 12.5 (Head angle)"]
        );
        t.host.p(bob).loc = Loc::new("minecraft:overworld", 3.5, 64.0, 4.5);
        assert_eq!(t.cmd(alice, "/coords Bob")[6], "Distance: 5");
        // Without getpos.others the argument is ignored.
        assert_eq!(t.cmd(bob, "/getpos Alice")[1], "X: 3 (+East <-> -West)");
        assert_eq!(t.console("getpos Bob")[0], "Current World: world");
        assert_eq!(t.console("getpos")[0], "Command Help: /getpos");
        t.host.p(bob).loc.world = "minecraft:the_nether".into();
        assert_eq!(t.console("getpos Bob")[0], "Current World: world_nether");
    }

    #[test]
    fn compass_and_depth() {
        let (mut t, alice, _bob) = server();
        assert_eq!(t.cmd(alice, "/compass"), ["Bearing: S (180 degrees)."]);
        t.host.p(alice).loc.yaw = -90.0;
        assert_eq!(t.cmd(alice, "/direction"), ["Bearing: E (90 degrees)."]);
        t.host.p(alice).loc.yaw = 150.0;
        assert_eq!(t.cmd(alice, "/compass"), ["Bearing: NW (330 degrees)."]);
        assert_eq!(t.cmd(alice, "/depth"), ["You are 1 block(s) above sea level."]);
        t.host.p(alice).loc.y = 63.2;
        assert_eq!(t.cmd(alice, "/height"), ["You are at sea level."]);
        t.host.p(alice).loc.y = 40.0;
        assert_eq!(t.cmd(alice, "/depth"), ["You are 23 block(s) below sea level."]);
        assert_eq!(t.console("compass"), ["Error: Only in-game players can use /compass."]);
    }

    #[test]
    fn gc_reports_server_stats() {
        let (mut t, _alice, _bob) = server();
        t.host.stats.worlds.push(("minecraft:the_nether".into(), 10, 2, 0));
        assert_eq!(
            t.console("gc"),
            [
                "Uptime: 1 hour",
                "Current TPS = 20",
                "Maximum memory: 2,048 MB.",
                "Allocated memory: 512 MB.",
                "Free memory: 1,536 MB.",
                "World \"world\": 441 chunks, 12 entities, 3 tiles.",
                "Nether \"world_nether\": 10 chunks, 2 entities, 0 tiles.",
            ]
        );
        t.host.stats.tps = 16.456;
        assert_eq!(t.console("tps")[1], "Current TPS = 16.46");
    }

    #[test]
    fn playtime_self_and_others() {
        let (mut t, alice, bob) = server();
        t.host.p(alice).stats.insert(PLAY.into(), 72_000 + 1_200);
        assert_eq!(t.cmd(alice, "/playtime"), ["Playtime: 1 hour 1 minute"]);
        t.host.p(bob).stats.insert(PLAY.into(), 20 * 60 * 60 * 26);
        assert_eq!(t.cmd(alice, "/playtime Bob"), ["Playtime of Bob: 1 day 2 hours"]);
        t.quit(bob);
        assert_eq!(t.console("playtime Bob"), ["Playtime of Bob: 1 day 2 hours"]);
        assert_eq!(t.console("playtime")[0], "Command Help: /playtime");
    }

    // ----- /whois /seen ------------------------------------------------------------------------------------------

    #[test]
    fn whois_shows_everything() {
        let (mut t, alice, bob) = server();
        let bob_ip = ip(&mut t, bob);
        t.host.p(bob).stats.insert(PLAY.into(), 72_000);
        t.host.p(bob).exp = 30;
        t.host.whitelist.push(bob);
        assert_eq!(
            t.cmd(alice, "/whois Bob"),
            [
                " ====== WhoIs: Bob ======".to_string(),
                " - Nick: Bob".into(),
                format!(" - UUID: {}", uuid_string(bob)),
                " - Health: 20/20".into(),
                " - Hunger: 20/20 (+5 saturation)".into(),
                " - Exp: 30 (Level 3)".into(),
                " - Location: (world, 0, 64, 0)".into(),
                " - Playtime: 1 hour".into(),
                " - Money: $0".into(),
                format!(" - IP Address: /{bob_ip}"),
                " - Gamemode: survival".into(),
                " - God mode: false".into(),
                " - OP: false".into(),
                " - Fly mode: false (not flying)".into(),
                " - Speed: 0.2".into(),
                " - Whitelist: true".into(),
                " - AFK: false".into(),
                " - Jail: false".into(),
                " - Muted: false".into(),
            ]
        );
        t.ess.users.user(bob).set("muted", json!(true));
        t.ess.users.user(bob).set("mute-reason", json!("rude"));
        assert_eq!(t.console("whois Bob").last().unwrap(), " - Muted: true Reason: rude");
        assert_eq!(t.console("whois Zed"), ["Error: Player not found."]);
    }

    #[test]
    fn whois_without_ip_node_hides_the_address() {
        let mut t = TestServer::new();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.whois]\n");
        let bob = t.join("Bob");
        let lines = t.cmd(bob, "/whois Bob");
        assert!(lines.iter().all(|l| !l.contains("IP Address")));
        assert_eq!(lines.len(), 18);
    }

    #[test]
    fn seen_offline_online_and_extras() {
        let (mut t, alice, bob) = server();
        let bob_ip = ip(&mut t, bob);
        let carol = t.join("Carol");
        t.host.now += 2 * HOUR;
        assert_eq!(t.cmd(carol, "/seen Bob"), ["Player Bob has been online since 2 hours."]);
        t.host.first_played.insert(bob, NOW);
        assert_eq!(
            t.cmd(alice, "/seen Bob"),
            [
                "Player Bob has been online since 2 hours.".to_string(),
                format!(" - UUID: {}", uuid_string(bob)),
                " - First login: November 14, 2023 at 10:13:20 PM UTC".into(),
                format!(" - IP Address: /{bob_ip}"),
            ]
        );
        t.quit(bob);
        t.host.now += 3 * HOUR;
        assert_eq!(t.cmd(carol, "/seen Bob"), ["Player Bob has been offline since 3 hours."]);
        t.console("tempban Bob 1d Rude");
        assert_eq!(
            t.cmd(alice, "/seen bob"),
            [
                "Player Bob has been offline since 3 hours.".to_string(),
                format!(" - UUID: {}", uuid_string(bob)),
                " - Whitelist: false".into(),
                " - Banned: Rude".into(),
                " - Ban expires: 1 day".into(),
                format!(" - IP Address: {bob_ip}"),
                " - Location: (world, 0, 64, 0)".into(),
            ]
        );
        assert_eq!(t.cmd(carol, "/seen Bob")[1], " - Banned: true");
    }

    #[test]
    fn seen_unknown_names_bans_and_ips() {
        let (mut t, alice, bob) = server();
        let bob_ip = ip(&mut t, bob);
        assert_eq!(t.cmd(alice, "/seen Zed"), ["Error: Player Zed was never on this server."]);
        t.host.bans.push(Ban { target: "Zed".into(), reason: "Hacks".into(), source: "Server".into(), expires: None });
        assert_eq!(t.cmd(alice, "/seen Zed"), [" - Banned: Hacks"]);
        t.host.ip_bans.push(Ban { target: "10.1.1.1".into(), reason: "x".into(), source: "Server".into(), expires: None });
        assert_eq!(
            t.cmd(alice, "/seen 10.1.1.1"),
            ["IP 10.1.1.1 is banned.", "Running search for players matching '10.1.1.1' (this could take a little while).", "No matching players found."]
        );
        let dave = t.join("Dave");
        t.host.p(dave).address = bob_ip.clone();
        t.ess.on_join(&mut t.host, dave);
        assert_eq!(
            t.cmd(alice, &format!("/seen {bob_ip}")),
            [
                format!("Running search for players matching '{bob_ip}' (this could take a little while)."),
                "The following players previously logged in from that IP address:".to_string(),
                "Bob, Dave".into()
            ]
        );
        assert_eq!(
            t.cmd(alice, "/alts Bob"),
            [
                "Running search for players matching 'Bob' (this could take a little while).".to_string(),
                "The following players previously logged in from that IP address:".into(),
                "Bob, Dave".into()
            ]
        );
        // Players without seen.alts get the normal answer.
        assert_eq!(t.cmd(bob, "/alts Dave")[0], "Player Dave has been online since now.");
        assert_eq!(t.cmd(alice, "/seen")[0], "Command Help: /seen");
    }

    // ----- /invsee ---------------------------------------------------------------------------------------------

    #[test]
    fn invsee_opens_a_copy_and_edits_with_modify() {
        let (mut t, alice, bob) = server();
        t.host.p(bob).inventory[0] = Some(Item::new("diamond", 3));
        t.host.p(bob).inventory[39] = Some(Item::new("diamond_helmet", 1));
        assert_eq!(t.cmd(alice, "/invsee Alice"), ["You can only view other players' inventories."]);
        assert_eq!(t.cmd(alice, "/invsee Bob"), none());
        let open = t.host.p(alice).open.clone().unwrap();
        assert_eq!(open.rows, 5);
        assert_eq!(open.title, "Bob");
        assert_eq!(open.items.len(), 45);
        assert_eq!(open.items[0], Some(Item::new("diamond", 3)));
        assert_eq!(open.items[39], Some(Item::new("diamond_helmet", 1)));
        let tag = open.tag.clone();
        assert_eq!(t.ess.on_container_click(&mut t.host, alice, &tag, 0), Allow::Yes);
        let mut items = open.items.clone();
        items[0] = None;
        items[5] = Some(Item::new("stone", 1));
        t.ess.on_container_close(&mut t.host, alice, &tag, &items);
        assert_eq!(t.host.p(bob).inventory[0], None);
        assert_eq!(t.host.p(bob).inventory[5], Some(Item::new("stone", 1)));
        // Equipment view: one row, read-only.
        t.cmd(alice, "/invsee Bob equip");
        let open = t.host.p(alice).open.clone().unwrap();
        assert_eq!((open.rows, open.title.as_str()), (1, "Equipped"));
        assert_eq!(open.items[3], Some(Item::new("diamond_helmet", 1)));
        assert_eq!(t.ess.on_container_click(&mut t.host, alice, &open.tag, 0), Allow::No);
    }

    #[test]
    fn invsee_without_modify_is_read_only() {
        let mut t = TestServer::new();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.invsee]\n");
        let carol = t.join("Carol");
        let bob = t.join("Bob");
        t.host.p(bob).inventory[0] = Some(Item::new("diamond", 3));
        t.cmd(carol, "/invsee Bob");
        let open = t.host.p(carol).open.clone().unwrap();
        assert_eq!(t.ess.on_container_click(&mut t.host, carol, &open.tag, 0), Allow::No);
        t.ess.on_container_close(&mut t.host, carol, &open.tag, &vec![None; 45]);
        assert_eq!(t.host.p(bob).inventory[0], Some(Item::new("diamond", 3)));
        // Other areas' containers are not ours.
        assert_eq!(t.ess.on_container_click(&mut t.host, carol, "kit:x", 0), Allow::Yes);
    }

    // ----- Jails ---------------------------------------------------------------------------------------------------

    fn jail_server(config: &str) -> (TestServer, Uuid, Uuid, Loc) {
        let mut t = TestServer::with_config(config);
        let alice = t.join("Alice");
        t.op("Alice");
        let bob = t.join("Bob");
        let cell = Loc::new("minecraft:overworld", 100.5, 40.0, 100.5);
        t.host.p(alice).loc = cell.clone();
        assert_eq!(t.cmd(alice, "/setjail Cell"), ["Jail Cell has been set."]);
        t.host.p(alice).loc = Loc::new("minecraft:overworld", 0.5, 64.0, 0.5);
        t.host.p(bob).loc = Loc::new("minecraft:overworld", 20.5, 64.0, 20.5);
        t.tick(1);
        t.host.said(Sender::Console);
        (t, alice, bob, cell)
    }

    #[test]
    fn setjail_jails_deljail() {
        let (mut t, alice, _bob, _cell) = jail_server("");
        let yaml = t.ess.read_yaml("jail.yml");
        assert_eq!(yaml["jails"]["cell"]["x"], json!(100.5));
        assert_eq!(yaml["jails"]["cell"]["world"], json!("minecraft:overworld"));
        assert_eq!(t.console("jails"), ["Jails: cell"]);
        t.cmd(alice, "/createjail other");
        assert_eq!(t.console("jails"), ["Jails: cell other"]);
        assert_eq!(t.console("deljail nope"), ["Error: That jail does not exist."]);
        assert_eq!(t.console("rmjail Other"), ["Jail Other has been removed."]);
        assert_eq!(t.console("deljail cell"), ["Jail cell has been removed."]);
        assert_eq!(t.console("jails"), ["No jails defined."]);
        assert_eq!(t.ess.read_yaml("jail.yml")["jails"], json!({}));
        assert_eq!(t.console("setjail x"), ["Error: Only in-game players can use /setjail."]);
    }

    #[test]
    fn jail_teleports_restricts_and_unjails_back() {
        let (mut t, alice, bob, cell) = jail_server("");
        t.said(alice);
        assert_eq!(t.console("jail Bob"), ["Player Bob jailed."]);
        assert_eq!(t.said(bob), ["You have been jailed!"]);
        assert_eq!(t.said(alice), ["Player Bob jailed by CONSOLE."]);
        assert_eq!(logs(&mut t).last().unwrap(), "Player Bob jailed by CONSOLE.");
        assert_eq!(t.loc(bob), cell);
        let data = t.userdata(bob);
        assert_eq!(data["jailed"], json!(true));
        assert_eq!(data["jail"], json!("cell"));
        assert_eq!(t.console("jailedplayers"), ["Jailed Players: Bob"]);
        // Restrictions.
        assert_eq!(t.cmd(bob, "/ping"), ["You do the crime, you do the time."]);
        assert_eq!(t.ess.on_block_break(&mut t.host, bob, "minecraft:overworld", [100, 39, 100], "minecraft:stone"), Allow::No);
        assert_eq!(t.ess.on_block_place(&mut t.host, bob, "minecraft:overworld", [100, 40, 101], "minecraft:stone"), Allow::No);
        assert_eq!(t.ess.on_interact(&mut t.host, bob, None, None), Allow::No);
        assert_eq!(t.ess.on_block_break(&mut t.host, alice, "minecraft:overworld", [0, 63, 0], "minecraft:stone"), Allow::Yes);
        assert_eq!(t.ess.on_respawn(&mut t.host, bob), Some(cell.clone()));
        assert_eq!(t.ess.on_respawn(&mut t.host, alice), None);
        // Already jailed elsewhere.
        assert_eq!(t.console("jail Bob other"), ["Person is already in jail: cell"]);
        // Release.
        assert_eq!(t.console("unjail Bob"), ["Player Bob unjailed."]);
        assert_eq!(t.said(bob), ["You have been released!"]);
        assert_eq!(t.loc(bob), Loc::new("minecraft:overworld", 20.5, 64.0, 20.5));
        assert_eq!(t.cmd(bob, "/ping"), ["Pong!"]);
        assert_eq!(t.console("jailedplayers"), ["There are no jailed players."]);
        assert_eq!(t.console("unjail Bob"), ["Command Help: /unjail", "Description: Jails/Unjails a player, TPs them to the jail specified.", "Usage(s);"]);
    }

    #[test]
    fn jail_allow_nodes_and_exempt() {
        let (mut t, _alice, bob, _cell) = jail_server("");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.jail.allow.ping, essentials.jail.allow-break]\n");
        t.console("jail Bob cell");
        t.said(bob);
        assert_eq!(t.cmd(bob, "/ping"), ["Pong!"]);
        assert_eq!(t.ess.on_block_break(&mut t.host, bob, "minecraft:overworld", [0, 0, 0], "minecraft:stone"), Allow::Yes);
        assert_eq!(t.console("jail Alice cell"), ["You may not jail that person!"]);
        assert_eq!(t.console("jail Bob nope"), ["Person is already in jail: cell"]);
        t.console("unjail Bob");
        assert_eq!(t.console("jail Bob nope"), ["Error: That jail does not exist."]);
    }

    #[test]
    fn timed_jail_releases_and_extends() {
        let (mut t, _alice, bob, _cell) = jail_server("");
        assert_eq!(t.console("jail Bob cell 10m"), ["Player Bob jailed for 10 minutes."]);
        assert_eq!(logs(&mut t).last().unwrap(), "Player Bob jailed for 10 minutes by CONSOLE.");
        t.said(bob);
        assert_eq!(t.cmd(bob, "/ping"), ["Player Bob jailed for 10 minutes."]);
        assert_eq!(t.console("jail Bob cell 1h"), ["Jail time extended to 1 hour."]);
        assert_eq!(logs(&mut t).last().unwrap(), "Player Bob jail's time extended to 1 hour by CONSOLE.");
        assert_eq!(t.console("whois Bob")[17], " - Jail: 1 hour");
        t.host.now += HOUR + 1;
        t.tick(20);
        assert_eq!(t.said(bob), ["You have been released."]);
        assert_eq!(t.loc(bob), Loc::new("minecraft:overworld", 20.5, 64.0, 20.5));
        assert!(!t.userdata(bob)["jailed"].as_bool().unwrap_or(false));
    }

    #[test]
    fn jail_online_time_counts_play_time_only() {
        let (mut t, _alice, bob, _cell) = jail_server("jail-online-time: true\nteleport-when-freed: spawn");
        t.host.p(bob).stats.insert(PLAY.into(), 1000);
        t.console("jail Bob cell 10m");
        let data = t.userdata(bob);
        assert_eq!(data["timestamps"]["jail"], json!(600_000));
        assert_eq!(data["timestamps"]["onlinejail"], json!(1000 + 12_000));
        t.said(bob);
        t.host.now += 2 * HOUR;
        t.tick(20);
        assert!(t.said(bob).is_empty());
        t.host.p(bob).stats.insert(PLAY.into(), 13_000);
        t.tick(20);
        assert_eq!(t.said(bob), ["You have been released."]);
        // teleport-when-freed: spawn.
        assert_eq!(t.loc(bob), Loc::new("minecraft:overworld", 0.5, 64.0, 0.5));
    }

    #[test]
    fn offline_jail_applies_on_join() {
        let (mut t, _alice, bob, cell) = jail_server("teleport-when-freed: 'off'");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.togglejail]\n");
        let carol = t.join("Carol");
        t.quit(bob);
        assert_eq!(t.cmd(carol, "/jail Bob cell"), ["You may not jail offline players."]);
        assert_eq!(t.console("jail Bob cell"), ["Player Bob jailed."]);
        t.join("Bob");
        assert_eq!(t.loc(bob), cell);
        assert_eq!(t.said(bob), ["You do the crime, you do the time."]);
        t.console("unjail Bob");
        // teleport-when-freed: off keeps them where they are.
        assert_eq!(t.loc(bob), cell);
    }

    // ----- Command hooks: cooldowns, log-console-commands, disabled-commands -----------------------------------------

    #[test]
    fn command_cooldowns_block_and_persist() {
        let mut t = TestServer::with_config("command-cooldowns:\n  ping: 60\n  '^ec.o( .*)?': 30\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/ping"), ["Pong!"]);
        assert_eq!(t.cmd(bob, "/ping"), ["You cannot type that command for 1 minute."]);
        assert_eq!(t.cmd(bob, "/eping hi"), ["You cannot type that command for 1 minute."]);
        let data = t.userdata(bob);
        let saved = data["timestamps"]["command-cooldowns"].as_array().unwrap().clone();
        assert_eq!(saved[0]["pattern"], json!("ping( .*)?"));
        assert_eq!(saved[0]["value"], json!(NOW + 60_000));
        assert_eq!(t.cmd(bob, "/echo a"), ["a"]);
        assert_eq!(t.cmd(bob, "/echo a"), ["You cannot type that command for 30 seconds."]);
        t.host.now += 61_000;
        assert_eq!(t.cmd(bob, "/ping"), ["Pong!"]);
        // Ops bypass (essentials.commandcooldowns.bypass).
        let alice = t.join("Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/ping"), ["Pong!"]);
        assert_eq!(t.cmd(alice, "/ping"), ["Pong!"]);
        // The console has no cooldowns.
        assert_eq!(t.console("ping"), ["Pong!"]);
        assert_eq!(t.console("ping"), ["Pong!"]);
    }

    #[test]
    fn command_cooldowns_without_persistence() {
        let mut t = TestServer::with_config("command-cooldowns:\n  ping: 60\ncommand-cooldown-persistence: false\n");
        let bob = t.join("Bob");
        t.cmd(bob, "/ping");
        assert_eq!(t.cmd(bob, "/ping"), ["You cannot type that command for 1 minute."]);
        assert!(t.userdata(bob)["timestamps"].get("command-cooldowns").is_none());
    }

    #[test]
    fn cooldown_patterns_match_like_java() {
        assert!(cooldown_matches("ping( .*)?", "ping"));
        assert!(cooldown_matches("ping( .*)?", "ping a b"));
        assert!(!cooldown_matches("ping( .*)?", "pings"));
        assert!(cooldown_matches(".*i.*( .*)?", "kit"));
        assert!(cooldown_matches("ban([^ip])( .*)?", "bans x"));
        assert!(!cooldown_matches("ban([^ip])( .*)?", "banip x"));
        assert!(cooldown_matches("(home|warp)s?( .*)?", "warps"));
        assert!(cooldown_matches("a+b*c?", "aaab"));
        assert!(cooldown_matches("[a-c]{2}\\d", "ab7"));
        assert!(!cooldown_matches("[a-c]{2}\\d", "ad7"));
    }

    #[test]
    fn console_commands_are_logged() {
        let mut t = TestServer::new();
        t.console("ping hi");
        assert_eq!(logs(&mut t), ["CONSOLE issued server command: /ping hi"]);
        let mut t = TestServer::with_config("log-console-commands: false");
        t.console("ping");
        assert!(logs(&mut t).is_empty());
    }

    #[test]
    fn disabled_commands_are_left_to_vanilla() {
        let mut t = TestServer::with_config("disabled-commands: [kick]");
        assert!(!t.ess.claims("kick Bob"));
        assert!(t.ess.claims("ban Bob"));
        assert_eq!(t.console("kick Bob"), ["The command kick is disabled."]);
    }

    // ----- /essentials /backup -------------------------------------------------------------------------------------------

    #[test]
    fn essentials_subcommands() {
        let (mut t, alice, _bob) = server();
        assert_eq!(
            t.cmd(alice, "/ess version")[..3],
            ["Server version: Horizon (Minecraft 26.3)".to_string(), "Brand version: Horizon".into(), "EssentialsX version: 2.21.0-horizon".into()]
        );
        assert_eq!(t.console("essentials commands"), ["EssentialsX is not relaying any commands to other plugins."]);
        assert_eq!(t.console("essentials debug"), ["Essentials 2.21.0-horizon debug mode enabled"]);
        assert_eq!(t.console("essentials debug off"), ["Essentials 2.21.0-horizon debug mode disabled"]);
        assert_eq!(t.console("essentials")[0], "Command Help: /essentials");
        assert_eq!(t.console("essentials nope")[0], "Command Help: /essentials");
        assert_eq!(t.cmd(alice, "/essentials moo").len(), 7);
        assert_eq!(t.console("essentials usermap cache"), ["Current cached users in user map is 2/2/0.", "There are 2 known users to the user cache with 2 name to UUID pairs."]);
    }

    #[test]
    fn essentials_reload_rereads_jails() {
        let (mut t, _alice, _bob, _cell) = jail_server("");
        t.ess.write_yaml("jail.yml", &json!({"jails": {"a": {"world": "minecraft:overworld", "x": 1.0, "y": 2.0, "z": 3.0}}}));
        t.console("essentials reload");
        assert_eq!(t.console("jails"), ["Jails: a"]);
    }

    #[test]
    fn essentials_cleanup_removes_old_users() {
        let (mut t, _alice, bob) = server();
        t.quit(bob);
        t.host.now += 40 * DAY;
        assert_eq!(t.console("essentials cleanup 30"), ["Cleaning userfiles.", "Userfiles Cleaned."]);
        assert!(!t.ess.users.exists(bob));
    }

    #[test]
    fn backup_runs_the_configured_command() {
        let mut t = TestServer::new();
        assert_eq!(t.console("backup"), ["Error: An external backup script has not been configured."]);
        let mut t = TestServer::with_config("backup:\n  command: 'save-all'\n");
        assert_eq!(t.console("backup"), ["Error: An external backup script has not been configured."]);
        let mut t = TestServer::with_config("backup:\n  command: 'true'\n");
        assert_eq!(t.console("backup"), ["Backup started."]);
        assert_eq!(t.host.console_commands, ["save-all", "save-off"]);
        assert_eq!(logs(&mut t), ["CONSOLE issued server command: /backup", "Backup started."]);
        for _ in 0..300 {
            t.tick(1);
            if t.host.console_commands.len() == 3 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(t.host.console_commands, ["save-all", "save-off", "save-on"]);
        assert_eq!(logs(&mut t), ["Backup finished."]);
    }
}
