// horizon-essentials area `social` (pure logic, standalone-tested): Chat & social: msg, r, mail, me, ignore, socialspy, helpop, broadcast*, the Chat module (formats, local/shout/question), afk, nick, realname, motd, rules, info, customtext, help, join/quit messages, mute.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    let _ = ess;
}

/// User.notifyOfMail: called by the core before each command of a player with essentials.mail.
pub fn notify_mail(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    let _ = (ess, host, player);
}

/// Whether Essentials replaces the vanilla join/quit lines (the glue then suppresses vanilla's).
pub fn join_messages_replaced(ess: &Ess) -> bool {
    let _ = ess;
    false
}

/// Every Essentials label as a greedy-string command (commands.json form), for the client's tab completion.
pub fn command_tree(ess: &Ess) -> serde_json::Value {
    let _ = ess;
    serde_json::json!({})
}

pub fn parse_date_diff(now: i64, text: &str, future: bool) -> Option<i64> {
    let _ = (now, text, future);
    None
}

pub fn format_date_diff(now: i64, date: i64) -> String {
    let _ = (now, date);
    String::new()
}

pub struct Regex;
impl Regex {
    pub fn new(pattern: &str) -> Option<Regex> {
        let _ = pattern;
        None
    }
    pub fn is_match(&self, s: &str) -> bool {
        let _ = s;
        false
    }
    pub fn find_in(&self, s: &str) -> bool {
        let _ = s;
        false
    }
}

pub fn block_url(s: &str) -> String {
    s.to_string()
}

#[cfg(test)]
mod tests {
    // L1 tests of the social area (included by ess_social.rs as its `tests` module).
    use super::*;
    use crate::game::ess_testkit::*;

    /// A server with these config lines; players joining through `j` have their join lines drained.
    fn srv(config: &str) -> TestServer {
        TestServer::with_config(config)
    }
    fn j(t: &mut TestServer, name: &str) -> Uuid {
        let p = t.join(name);
        t.said(p);
        p
    }
    fn console(t: &mut TestServer) -> Vec<String> {
        t.host.said(Sender::Console)
    }
    fn replaced(chat: &Chat) -> Vec<(Option<Vec<Uuid>>, String)> {
        match chat {
            Chat::Replace(lines) => lines.iter().map(|(r, s)| (r.clone(), crate::game::ess_text::plain(s))).collect(),
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    // ----- msg / r / msgtoggle / rtoggle / socialspy ----------------------------------------------------

    #[test]
    fn msg_and_reply_between_players() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        assert_eq!(t.cmd(alice, "/msg Bob hello there"), ["[me -> Bob] hello there"]);
        assert_eq!(t.said(bob), ["[Alice -> me] hello there"]);
        assert_eq!(t.cmd(bob, "/r hi back"), ["[me -> Alice] hi back"]);
        assert_eq!(t.said(alice), ["[Bob -> me] hi back"]);
        // Aliases.
        assert_eq!(t.cmd(alice, "/tell bob yo"), ["[me -> Bob] yo"]);
        assert_eq!(t.cmd(alice, "/w Bob yo"), ["[me -> Bob] yo"]);
        assert_eq!(t.cmd(bob, "/reply ok"), ["[me -> Alice] ok"]);
    }

    #[test]
    fn msg_errors_and_console() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        assert_eq!(t.cmd(alice, "/r hi"), ["Error: You have nobody to whom you can reply."]);
        assert_eq!(t.cmd(alice, "/msg Nobody hi"), ["Error: Player not found."]);
        assert_eq!(t.cmd(alice, "/msg Bob")[0], "Command Help: /msg");
        assert_eq!(t.cmd(alice, "/msg console hi"), ["[me -> Console] hi"]);
        assert_eq!(console(&mut t), ["[Alice -> me] hi"]);
        assert_eq!(t.console("msg Alice from console"), ["[me -> Alice] from console"]);
        assert_eq!(t.said(alice), ["[Console -> me] from console"]);
        // The console replies to its last correspondent.
        assert_eq!(t.console("r again"), ["[me -> Alice] again"]);
    }

    #[test]
    fn msg_colors_need_permission() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        // The default group has essentials.msg.color but not .format.
        t.cmd(alice, "/msg Bob &chi &lbold");
        let spans = t.host.spans.get(&Sender::Player(bob)).unwrap().last().unwrap().clone();
        let text = crate::game::ess_text::plain(&spans);
        assert_eq!(text, "[Alice -> me] hi &lbold");
        assert!(spans.iter().any(|s| s.text.starts_with("hi") && s.style.color.as_deref() == Some("red")));
    }

    #[test]
    fn reply_goes_to_the_first_correspondent_within_the_timeout() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        let carl = j(&mut t, "Carl");
        t.cmd(alice, "/msg Bob one");
        t.cmd(carl, "/msg Bob two");
        t.said(bob);
        assert_eq!(t.cmd(bob, "/r back"), ["[me -> Alice] back"]);
        // rtoggle: always the last sender.
        assert_eq!(t.cmd(bob, "/rtoggle"), ["You do not have access to that command."]);
        t.op("Bob");
        assert_eq!(t.cmd(bob, "/rtoggle"), ["Replying to last message recipient disabled."]);
        t.cmd(carl, "/msg Bob three");
        t.said(bob);
        assert_eq!(t.cmd(bob, "/r back"), ["[me -> Carl] back"]);
        assert_eq!(t.cmd(bob, "/rtoggle on"), ["Replying to last message recipient enabled."]);
    }

    #[test]
    fn reply_target_offline() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.cmd(alice, "/msg Bob hi");
        t.quit(alice);
        assert_eq!(t.cmd(bob, "/r hi"), ["Alice recently went offline."]);
    }

    #[test]
    fn msgtoggle_blocks_messages() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Bob");
        assert_eq!(t.cmd(bob, "/msgtoggle"), ["Receiving messages disabled."]);
        assert_eq!(t.cmd(alice, "/msg Bob hi"), ["Bob has messages disabled."]);
        assert!(t.said(bob).is_empty());
        assert_eq!(t.cmd(bob, "/msgtoggle"), ["Receiving messages enabled."]);
        // Others.
        assert_eq!(t.cmd(bob, "/msgtoggle Alice"), ["Receiving messages disabled for Alice."]);
        assert_eq!(t.said(alice), ["Receiving messages disabled."]);
        // An op sender bypasses (essentials.msgtoggle.bypass).
        assert_eq!(t.cmd(bob, "/msg Alice hi"), ["[me -> Alice] hi"]);
    }

    #[test]
    fn socialspy_sees_private_messages() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        let carl = j(&mut t, "Carl");
        t.op("Carl");
        assert_eq!(t.cmd(carl, "/socialspy"), ["SocialSpy for Carl: enabled"]);
        t.cmd(alice, "/msg Bob secret");
        assert_eq!(t.said(carl), ["[SS] [Alice -> Bob] secret"]);
        // Spy on socialspy-commands (mail is one).
        t.cmd(alice, "/mail send Bob hey");
        assert_eq!(t.said(carl), ["[SS] Alice: /mail send Bob hey"]);
        let _ = bob;
        assert_eq!(t.cmd(carl, "/socialspy off"), ["SocialSpy for Carl: disabled"]);
        t.cmd(alice, "/msg Bob secret");
        assert!(t.said(carl).is_empty());
        assert_eq!(t.userdata(carl)["socialspy"], false);
    }

    #[test]
    fn socialspy_without_permission_is_reset_on_join() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        t.ess.users.user(alice).set("socialspy", serde_json::json!(true));
        t.quit(alice);
        j(&mut t, "Alice");
        assert_eq!(t.userdata(alice)["socialspy"], false);
    }

    // ----- ignore -----------------------------------------------------------------------------------------

    #[test]
    fn ignore_filters_msg_chat_and_mail() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Bob");
        assert_eq!(t.cmd(bob, "/ignore"), ["You are not ignoring anyone."]);
        assert_eq!(t.cmd(bob, "/ignore Alice"), ["You ignore player Alice from now on."]);
        assert_eq!(t.cmd(bob, "/ignore"), ["Ignored: Alice"]);
        assert_eq!(t.cmd(bob, "/ignore Bob"), ["Ignoring yourself won't solve your problems."]);
        // msg: dropped silently.
        assert_eq!(t.cmd(alice, "/msg Bob hi"), Vec::<String>::new());
        assert!(t.said(bob).is_empty());
        // chat: Bob is not a recipient.
        let lines = replaced(&t.chat(alice, "hello"));
        assert_eq!(lines, vec![(Some(vec![alice]), "<Alice> hello".to_string())]);
        // mail: "sent" but not delivered.
        assert_eq!(t.cmd(alice, "/mail send Bob hey"), ["Bob has been sent the following mail:", "hey"]);
        assert_eq!(t.cmd(bob, "/mail read"), ["You do not have any mail."]);
        assert_eq!(t.cmd(bob, "/unignore Alice"), ["You are not ignoring player Alice anymore."]);
        assert_eq!(t.chat(alice, "hello"), Chat::Pass);
    }

    #[test]
    fn ignore_exempt_and_userdata() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.cmd(bob, "/ignore Alice");
        let data = t.userdata(bob);
        assert_eq!(data["ignore"][0], uuid_string(alice));
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.ignore, essentials.msg, essentials.chat.ignoreexempt]\n");
        assert_eq!(t.cmd(bob, "/ignore Alice"), ["You may not ignore that player."]);
    }

    // ----- afk --------------------------------------------------------------------------------------------

    #[test]
    fn afk_toggle_and_broadcast() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        assert_eq!(t.cmd(alice, "/afk"), ["You are now AFK."]);
        assert_eq!(t.said(bob), ["* Alice is now AFK."]);
        assert_eq!(t.userdata(alice)["afk"], true);
        assert_eq!(t.cmd(bob, "/msg Alice hi"), ["Alice is currently AFK and may not respond.", "[me -> Alice] hi"]);
        t.said(alice);
        assert_eq!(t.cmd(alice, "/away"), ["You are no longer AFK."]);
        assert_eq!(t.said(bob), ["* Alice is no longer AFK."]);
    }

    #[test]
    fn afk_messages_need_permission() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        assert_eq!(t.cmd(alice, "/afk lunch"), ["Error: You don't have permission to set an AFK message."]);
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/afk lunch"), ["You are now AFK."]);
        t.said(bob);
        assert_eq!(t.cmd(bob, "/msg Alice hi"), ["Alice is currently AFK and may not respond: lunch", "[me -> Alice] hi"]);
        // essentials.afk.others: an op toggles another player.
        assert_eq!(t.cmd(alice, "/afk Bob"), ["* Bob is now AFK."]);
        assert_eq!(t.said(bob), ["You are now AFK."]);
    }

    #[test]
    fn afk_others_from_console() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        assert_eq!(t.console("afk Alice"), Vec::<String>::new());
        assert_eq!(t.said(alice), ["You are now AFK."]);
        assert_eq!(t.console("afk")[0], "Command Help: /afk");
    }

    #[test]
    fn auto_afk_after_inactivity() {
        let mut t = srv("auto-afk: 60");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.tick(1100);
        assert!(t.said(alice).is_empty());
        t.tick(200);
        assert!(t.said(alice).contains(&"You are now AFK.".to_string()));
        assert!(t.said(bob).contains(&"* Alice is now AFK.".to_string()));
        // Moving more than 3 blocks from the AFK spot ends it.
        let mut to = t.loc(alice);
        to.x += 5.0;
        t.walk(alice, to);
        t.tick(1);
        assert_eq!(t.said(alice), ["You are no longer AFK."]);
    }

    #[test]
    fn auto_afk_kick_and_commands() {
        let mut t = srv("auto-afk: 0\nauto-afk-timeout: 30");
        let alice = j(&mut t, "Alice");
        t.tick(700);
        assert_eq!(t.host.kicked, vec![(alice, "You have been kicked for idling more than 0.5 minutes.".to_string())]);

        let mut t = srv("auto-afk: 0\nauto-afk-timeout: 60\nafk-timeout-commands: ['say {USERNAME} idled {KICKTIME}']");
        let _alice = j(&mut t, "Alice");
        t.tick(1300);
        assert!(t.host.kicked.is_empty());
        assert_eq!(t.host.console_commands, ["say Alice idled 1.0"]);
    }

    #[test]
    fn afk_freeze_keeps_players_in_place() {
        let mut t = srv("freeze-afk-players: true");
        let alice = j(&mut t, "Alice");
        t.cmd(alice, "/afk");
        let start = t.loc(alice);
        let mut to = start.clone();
        to.x += 5.0;
        t.walk(alice, to);
        t.tick(1);
        assert_eq!(t.loc(alice).x, start.x);
        assert!(t.said(alice).is_empty());
    }

    #[test]
    fn afk_ends_on_chat_and_commands() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        t.cmd(alice, "/afk");
        let _ = t.chat(alice, "back");
        t.tick(1);
        assert_eq!(t.said(alice), ["You are no longer AFK."]);
        t.cmd(alice, "/afk");
        assert_eq!(t.cmd(alice, "/ping"), ["You are no longer AFK.", "Pong!"]);
    }

    #[test]
    fn afk_list_name() {
        let mut t = srv("afk-list-name: '[AFK] {USERNAME}'");
        let alice = j(&mut t, "Alice");
        t.cmd(alice, "/afk");
        assert_eq!(t.host.p(alice).tab_name.as_deref(), Some("[AFK] Alice"));
        t.cmd(alice, "/afk");
        assert_eq!(t.host.p(alice).tab_name, None);
    }

    // ----- broadcast / broadcastworld / me / helpop ------------------------------------------------------

    #[test]
    fn broadcast_to_everyone() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/broadcast Hello &cworld"), ["[Broadcast] Hello world"]);
        assert_eq!(t.said(bob), ["[Broadcast] Hello world"]);
        let spans = t.host.spans.get(&Sender::Player(bob)).unwrap().last().unwrap().clone();
        assert!(spans.iter().any(|s| s.text == "world" && s.style.color.as_deref() == Some("red")));
        t.console("bc from console");
        assert_eq!(t.said(bob), ["[Broadcast] from console"]);
    }

    #[test]
    fn broadcastworld_only_to_that_world() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        t.host.p(bob).loc = Loc::new("minecraft:the_nether", 0.0, 64.0, 0.0);
        assert_eq!(t.cmd(alice, "/bcw hi all"), ["[Broadcast] hi all"]);
        assert!(t.said(bob).is_empty());
        assert!(t.cmd(alice, "/broadcastworld world_nether hi nether").is_empty());
        assert_eq!(t.said(bob), ["[Broadcast] hi nether"]);
        assert_eq!(t.console("broadcastworld nowhere hi"), ["Error: Invalid world."]);
    }

    #[test]
    fn me_action() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        assert_eq!(t.cmd(alice, "/me waves"), ["* Alice waves"]);
        assert_eq!(t.said(bob), ["* Alice waves"]);
        t.console("me speaks");
        assert_eq!(t.said(bob), ["* @ speaks"]);
    }

    #[test]
    fn me_in_local_radius() {
        let mut t = srv("chat:\n  radius: 10");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.host.p(bob).loc = Loc::new("minecraft:overworld", 100.0, 64.0, 0.0);
        assert_eq!(t.cmd(alice, "/me waves"), ["* Alice waves"]);
        assert!(t.said(bob).is_empty());
    }

    #[test]
    fn helpop_reaches_receivers() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Bob");
        assert_eq!(t.cmd(alice, "/helpop I am stuck"), ["[HelpOp] Alice: I am stuck"]);
        assert_eq!(t.said(bob), ["[HelpOp] Alice: I am stuck"]);
        assert_eq!(console(&mut t), ["[HelpOp] Alice: I am stuck"]);
    }

    // ----- mute -------------------------------------------------------------------------------------------

    #[test]
    fn mute_blocks_chat_msg_mail_me() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/mute Bob"), ["Player Bob muted.", "Alice has muted player Bob."]);
        assert_eq!(t.said(bob), ["You have been muted!"]);
        assert_eq!(t.userdata(bob)["muted"], true);
        console(&mut t);
        assert_eq!(t.chat(bob, "hello"), Chat::Cancel);
        assert_eq!(t.said(bob), ["Your voice has been silenced!"]);
        assert_eq!(console(&mut t), ["Bob tried to speak, but is muted: hello"]);
        assert_eq!(t.cmd(bob, "/msg Alice hi"), ["Your voice has been silenced!"]);
        assert!(t.said(alice).is_empty());
        assert_eq!(t.cmd(bob, "/mail send Alice hi"), ["Error: Your voice has been silenced!"]);
        assert_eq!(t.cmd(bob, "/me waves"), ["Error: Your voice has been silenced!"]);
        assert_eq!(t.cmd(alice, "/unmute Bob"), ["Player Bob unmuted.", "Alice unmuted Bob."]);
        assert_eq!(t.said(bob), ["You have been unmuted."]);
        assert_eq!(t.chat(bob, "hello"), Chat::Pass);
        assert_eq!(t.cmd(alice, "/unmute Bob"), ["Bob is not muted."]);
    }

    #[test]
    fn mute_for_a_time_with_reason() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/mute Bob 10m spam"), ["Player Bob muted for 10 minutes. Reason: spam", "Alice has muted player Bob for 10 minutes. Reason: spam"]);
        assert_eq!(t.said(bob), ["You have been muted for 10 minutes. Reason: spam"]);
        assert_eq!(t.userdata(bob)["mute-reason"], "spam");
        assert_eq!(t.cmd(bob, "/msg Alice hi"), ["Your voice has been silenced for 10 minutes! Reason: spam"]);
        t.host.now += 10 * 60 * 1000;
        t.tick(50);
        assert_eq!(t.said(bob), ["You can now talk again."]);
        assert_eq!(t.chat(bob, "hi"), Chat::Pass);
    }

    #[test]
    fn mute_exempt_offline_and_unknown() {
        let mut t = srv("max-mute-time: 60");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.mute.exempt]\n");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/mute Bob"), ["Error: You may not mute that player."]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [-essentials.mute.unlimited]\n");
        assert_eq!(t.cmd(alice, "/mute Bob 2h"), ["You may not mute a player for this period of time."]);
        t.quit(bob);
        assert_eq!(t.cmd(alice, "/mute Bob"), ["Player Bob muted.", "Alice has muted player Bob."]);
        assert_eq!(t.console("mute Ghost")[0], "Warning: The user 'Ghost' has never joined this server.");
    }

    #[test]
    fn mute_commands_are_blocked() {
        let mut t = srv("mute-commands: [ping]");
        let alice = j(&mut t, "Alice");
        t.console("mute Alice");
        t.said(alice);
        assert_eq!(t.cmd(alice, "/ping"), ["Your voice has been silenced!"]);
    }

    // ----- nick / realname --------------------------------------------------------------------------------

    #[test]
    fn nick_set_and_off() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/nick Ali"), ["Your nickname is now ~Ali."]);
        assert_eq!(t.userdata(alice)["nickname"], "Ali");
        assert_eq!(t.host.p(alice).display_name.as_deref(), Some("~Ali"));
        let lines = replaced(&t.chat(alice, "hi"));
        assert_eq!(lines[0].1, "<~Ali> hi");
        assert_eq!(t.cmd(bob, "/realname Ali"), ["~Ali is Alice"]);
        assert_eq!(t.cmd(alice, "/nick off"), ["You no longer have a nickname."]);
        assert_eq!(t.host.p(alice).display_name.as_deref(), Some("Alice"));
        assert_eq!(t.cmd(bob, "/realname zzz"), ["Error: Player not found."]);
    }

    #[test]
    fn nick_rules() {
        let mut t = srv("max-nick-length: 5\nnick-blacklist: ['^Din']");
        let alice = j(&mut t, "Alice");
        let _bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/nick Al!x"), ["Error: Nicknames must be alphanumeric."]);
        assert_eq!(t.cmd(alice, "/nick Abcdef"), ["Error: That nickname is too long."]);
        let lines = t.cmd(alice, "/nick Bob");
        assert_eq!(lines[0], "Command Help: /nick");
        assert_eq!(lines.last().unwrap(), "That name is already in use.");
        // Ops bypass the blacklist; players don't.
        assert_eq!(t.cmd(alice, "/nick Dinne"), ["Your nickname is now ~Dinne."]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.nick]\n");
        let carl = j(&mut t, "Carl");
        assert_eq!(t.cmd(carl, "/nick Dinne"), ["Error: That nickname is not allowed."]);
        // Colors only with essentials.nick.color.
        assert_eq!(t.cmd(carl, "/nick &cCar"), ["Error: Nicknames must be alphanumeric."]);
        assert_eq!(t.cmd(alice, "/nick &cAl"), ["Your nickname is now ~Al."]);
    }

    #[test]
    fn nick_others_and_console() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/nick Bob Bobby"), ["Nickname changed."]);
        assert_eq!(t.said(bob), ["Your nickname is now ~Bobby."]);
        assert_eq!(t.console("nick Bob off"), ["Nickname changed."]);
        assert_eq!(t.said(bob), ["You no longer have a nickname."]);
        // The own name with other casing: no prefix.
        assert_eq!(t.cmd(alice, "/nick ALICE"), ["Your nickname is now ALICE."]);
    }

    #[test]
    fn ops_name_color_and_group_prefix() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        t.op("Alice");
        let lines = match t.chat(alice, "hi") {
            Chat::Replace(lines) => lines,
            other => panic!("{other:?}"),
        };
        assert!(lines[0].1.iter().any(|s| s.text == "Alice" && s.style.color.as_deref() == Some("dark_red")));
        t.permissions("groups:\n  default:\n    default: true\n  admin:\n    prefix: '&4[Admin] '\n    permissions: []\nusers:\n  bob:\n    groups: [admin]\n");
        let bob = j(&mut t, "Bob");
        let lines = replaced(&t.chat(bob, "hey"));
        assert_eq!(lines[0].1, "<[Admin] Bob> hey");
    }

    // ----- chat -------------------------------------------------------------------------------------------

    #[test]
    fn plain_chat_passes_as_vanilla() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let _bob = j(&mut t, "Bob");
        assert_eq!(t.chat(alice, "hello"), Chat::Pass);
    }

    #[test]
    fn chat_format_and_placeholders() {
        let mut t = srv("chat:\n  format: '&7[{GROUP}]&r {DISPLAYNAME}&7:&r {MESSAGE} ({WORLD} {SHORTWORLDNAME} {USERNAME} {NICKNAME})'");
        let alice = j(&mut t, "Alice");
        let lines = replaced(&t.chat(alice, "hi"));
        assert_eq!(lines, vec![(None, "[default] Alice: hi (world W Alice Alice)".to_string())]);
    }

    #[test]
    fn group_formats_and_world_aliases() {
        let mut t = srv("chat:\n  group-formats:\n    admin: '{WORLD} [A] {DISPLAYNAME}: {MESSAGE}'\n  world-aliases:\n    world: '&aO'");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.chat.color]\n  admin:\n    permissions: []\nusers:\n  bob:\n    groups: [admin]\n");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        assert_eq!(t.chat(alice, "hi"), Chat::Pass);
        assert_eq!(replaced(&t.chat(bob, "hi")), vec![(None, "O [A] Bob: hi".to_string())]);
    }

    #[test]
    fn chat_colors_and_urls_by_permission() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let lines = match t.chat(alice, "&chi") {
            Chat::Replace(lines) => lines,
            other => panic!("{other:?}"),
        };
        assert!(lines[0].1.iter().any(|s| s.text == "hi" && s.style.color.as_deref() == Some("red")));
        assert_eq!(replaced(&t.chat(alice, "see google.com now"))[0].1, "<Alice> see google com now");
        assert_eq!(t.chat(alice, "&c"), Chat::Cancel);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: []\n");
        assert_eq!(t.chat(alice, "&chi"), Chat::Pass);
    }

    #[test]
    fn local_chat_radius_and_spy() {
        let mut t = srv("chat:\n  radius: 10");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        let carl = j(&mut t, "Carl");
        t.op("Carl");
        t.host.p(bob).loc = Loc::new("minecraft:overworld", 100.0, 64.0, 0.0);
        t.host.p(carl).loc = Loc::new("minecraft:overworld", 200.0, 64.0, 0.0);
        let lines = replaced(&t.chat(alice, "hi"));
        assert_eq!(lines[0], (Some(vec![alice]), "[L] <Alice> hi".to_string()));
        assert_eq!(lines[1], (Some(vec![carl]), "<Alice> hi".to_string()));
        // Shout reaches everyone.
        let lines = replaced(&t.chat(alice, "!hello all"));
        assert_eq!(lines[0].0, None);
        assert_eq!(lines[0].1, "[Shout] <Alice> hello all");
        let lines = replaced(&t.chat(alice, "?anyone"));
        assert_eq!(lines[0].1, "[Question] <Alice> anyone");
    }

    #[test]
    fn shout_and_question_need_permission() {
        let mut t = srv("chat:\n  radius: 10");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [-essentials.chat.shout, -essentials.chat.question, -essentials.chat.local]\n");
        let alice = j(&mut t, "Alice");
        assert_eq!(t.chat(alice, "!hi"), Chat::Cancel);
        assert_eq!(t.said(alice), ["You don't have permission to shout."]);
        assert_eq!(t.chat(alice, "?hi"), Chat::Cancel);
        assert_eq!(t.said(alice), ["You don't have permission to send question messages."]);
        assert_eq!(t.chat(alice, "hi"), Chat::Cancel);
        assert_eq!(t.said(alice), ["You don't have permission to speak in local chat."]);
    }

    #[test]
    fn toggleshout_makes_chat_shout() {
        let mut t = srv("chat:\n  radius: 10");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        t.host.p(bob).loc = Loc::new("minecraft:overworld", 100.0, 64.0, 0.0);
        assert_eq!(t.cmd(alice, "/toggleshout"), ["Shout mode enabled."]);
        let lines = replaced(&t.chat(alice, "hi"));
        assert_eq!(lines, vec![(None, "[Shout] <Alice> hi".to_string())]);
        assert_eq!(t.cmd(alice, "/toggleshout off"), ["Shout mode disabled."]);
    }

    // ----- mail -------------------------------------------------------------------------------------------

    fn mail_date(ms: i64) -> String {
        use chrono::TimeZone;
        chrono::Local.timestamp_millis_opt(ms).unwrap().format("%Y/%m/%d %H:%M").to_string()
    }

    #[test]
    fn mail_send_notify_read_clear() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.quit(bob);
        assert_eq!(t.cmd(alice, "/mail send Bob Hello there"), ["Bob has been sent the following mail:", "Hello there"]);
        let sent = t.host.now;
        let bob = t.join("Bob");
        let lines = t.said(bob);
        assert!(lines.contains(&"You have 1 messages! Type /mail read to view your mail.".to_string()), "{lines:?}");
        assert_eq!(
            t.cmd(bob, "/mail read"),
            [" ---- Mail: read -- Page 1/1 ----".to_string(), format!("[{}] [Alice] Hello there", mail_date(sent)), "To clear your mail, type /mail clear.".to_string()]
        );
        let data = t.userdata(bob);
        assert_eq!(data["mail"][0]["read"], true);
        assert_eq!(data["mail"][0]["sender-name"], "Alice");
        assert_eq!(data["mail"][0]["sender-uuid"], uuid_string(alice));
        assert_eq!(t.cmd(bob, "/mail clear"), ["Mail cleared!"]);
        assert_eq!(t.cmd(bob, "/mail read"), ["You do not have any mail."]);
        assert_eq!(t.cmd(alice, "/mail send Nobody hi"), ["Error: Player Nobody was never on this server."]);
        assert_eq!(t.cmd(alice, "/mail")[0], "Command Help: /mail");
    }

    #[test]
    fn mail_notify_on_command_with_cooldown() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.cmd(alice, "/mail send Bob one");
        assert_eq!(t.cmd(bob, "/ping"), ["You have 1 messages! Type /mail read to view your mail.", "Pong!"]);
        assert_eq!(t.cmd(bob, "/ping"), ["Pong!"]);
        t.host.now += 61_000;
        assert_eq!(t.cmd(bob, "/ping"), ["You have 1 messages! Type /mail read to view your mail.", "Pong!"]);
    }

    #[test]
    fn mail_no_new_mail_on_join() {
        let mut t = srv("");
        let alice = t.join("Alice");
        assert!(t.said(alice).contains(&"You have no new mail.".to_string()));
        let mut t = srv("notify-no-new-mail: false");
        let alice = t.join("Alice");
        assert!(!t.said(alice).contains(&"You have no new mail.".to_string()));
    }

    #[test]
    fn mail_sendtemp_expires() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/mail sendtemp Bob 1m soon gone"), ["Bob has been sent the following mail which will expire in 1 minute:", "soon gone"]);
        let lines = t.cmd(bob, "/mail read");
        assert!(lines[1].starts_with("[⚠] ["), "{lines:?}");
        t.cmd(alice, "/mail sendtemp Bob 1m again");
        t.host.now += 120_000;
        assert_eq!(t.cmd(bob, "/mail read"), ["You do not have any mail."]);
    }

    #[test]
    fn mail_rate_limit_and_sendall() {
        let mut t = srv("mails-per-minute: 1");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.cmd(alice, "/mail send Bob one");
        assert_eq!(t.cmd(alice, "/mail send Bob two"), ["Error: Too many mails have been sent within the last minute. Maximum: 1"]);
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/mail sendall to all"), ["Mail sent!"]);
        let data = t.userdata(bob);
        assert_eq!(data["mail"][0]["message"], "to all");
        assert_eq!(t.userdata(alice)["mail"][0]["message"], "to all");
        assert_eq!(t.cmd(alice, "/mail clearall"), ["Mail cleared for all players!"]);
        assert_eq!(t.cmd(bob, "/mail read"), ["You do not have any mail."]);
    }

    #[test]
    fn mail_clear_index_others_and_console() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let bob = j(&mut t, "Bob");
        t.op("Alice");
        assert_eq!(t.console("mail Bob first"), ["Mail sent!"]);
        assert_eq!(t.console("mail send Bob second"), ["Mail sent!"]);
        assert_eq!(t.cmd(bob, "/mail clear 5"), ["You must specify a number between 1-2."]);
        assert_eq!(t.cmd(bob, "/mail clear 1"), ["Mail cleared!"]);
        assert_eq!(t.userdata(bob)["mail"][0]["message"], "first");
        assert_eq!(t.userdata(bob)["mail"][0]["sender-name"], "Console");
        assert_eq!(t.cmd(alice, "/mail clear Bob"), ["Mail cleared!"]);
        assert_eq!(t.cmd(alice, "/mail clear Bob"), ["Bob does not have any mail."]);
        assert_eq!(t.console("mail read"), ["Error: Only in-game players can use mail read."]);
    }

    // ----- text files: motd, rules, info, customtext, help ------------------------------------------------

    #[test]
    fn motd_on_join_and_command() {
        let mut t = srv("");
        let alice = t.join("Alice");
        let lines = t.said(alice);
        assert_eq!(&lines[..4], ["Welcome, Alice!", "Type /help for a list of commands.", "Type /list to see who else is online.", "Players online: 1 - World time: 6:00 AM"]);
        let lines = t.cmd(alice, "/motd");
        assert_eq!(lines[0], " ---- Motd -- Page 1/1 ----");
        assert_eq!(lines[1], "Welcome, Alice!");
    }

    #[test]
    fn motd_delay() {
        let mut t = srv("delay-motd: 1000");
        let alice = t.join("Alice");
        assert!(!t.said(alice).contains(&"Welcome, Alice!".to_string()));
        t.tick(20);
        assert_eq!(t.said(alice)[0], "Welcome, Alice!");
    }

    #[test]
    fn rules_info_and_chapters() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        assert_eq!(t.cmd(alice, "/rules"), [" ---- Rules -- Page 1/1 ----", "[1] Be respectful", "[2] Be ethical", "[3] Use common sense"]);
        let lines = t.cmd(alice, "/info");
        assert_eq!(lines[0], " ---- Info -- Page 1/3 ----");
        assert_eq!(lines[1], "This is the info file.");
        assert_eq!(lines.last().unwrap(), "Type /info 2 to read the next page.");
        let lines = t.cmd(alice, "/info chapter1");
        assert_eq!(lines[0], " ---- Info: chapter1 -- Page 1 of 1 ----");
        assert_eq!(lines[1], "Lines starting with # begin a new chapter");
        assert_eq!(t.cmd(alice, "/info nothing"), ["Unknown chapter."]);
        assert_eq!(t.cmd(alice, "/info 9"), ["Unknown chapter."]);
        let tags = t.cmd(alice, "/info tags");
        assert!(tags.contains(&"Player's user name: Alice".to_string()), "{tags:?}");
        assert!(tags.contains(&"Player's primary group: default".to_string()));
        let tags = t.cmd(alice, "/info tags 2");
        assert_eq!(tags[0], " ---- Info: tags -- Page 2 of 3 ----");
        assert!(tags.contains(&"Player's coordinates: 0, 64, 0".to_string()), "{tags:?}");
    }

    #[test]
    fn chapter_index_when_file_starts_with_a_chapter() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        std::fs::write(t.ess.dir.join("rules.txt"), "#Build\nno griefing\n#Chat\nbe nice\n").unwrap();
        assert_eq!(t.cmd(alice, "/rules"), ["Select chapter:", "Build, Chat"]);
        assert_eq!(t.cmd(alice, "/rules chat"), [" ---- Rules: chat -- Page 1 of 1 ----", "be nice"]);
    }

    #[test]
    fn per_player_and_group_text_files() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        std::fs::write(t.ess.dir.join("rules_alice.txt"), "only for alice\n").unwrap();
        assert_eq!(t.cmd(alice, "/rules"), [" ---- Rules -- Page 1/1 ----", "only for alice"]);
        std::fs::write(t.ess.dir.join("info_default.txt"), "group {GROUP}\n").unwrap();
        assert_eq!(t.cmd(alice, "/info"), [" ---- Info -- Page 1/1 ----", "group default"]);
    }

    #[test]
    fn customtext_chapters() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/customtext vote"), ["If you add the correct alias to commands.yml", "This text here will be shown if you type /vote", ""]);
    }

    #[test]
    fn help_lists_and_searches_commands() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let lines = t.cmd(alice, "/help ignore");
        assert_eq!(lines[0], " ---- Help: ignore -- Page 1/1 ----");
        assert_eq!(lines[1], "Commands matching \"ignore\":");
        assert!(lines.contains(&"/ignore: Ignore or unignore other players.".to_string()), "{lines:?}");
        let lines = t.cmd(alice, "/help");
        assert!(lines[0].starts_with(" ---- Help -- Page 1/"), "{lines:?}");
        assert_eq!(t.console("help"), ["To view help from the console, type '?'."]);
        let lines = t.cmd(alice, "/help /msg");
        assert_eq!(lines[0], "Command Help: /msg");
        assert_eq!(lines[1], "Description: Sends a private message to the specified player.");
    }

    // ----- join / quit messages ---------------------------------------------------------------------------

    #[test]
    fn custom_join_and_quit_messages() {
        let mut t = srv("custom-join-message: '&e{PLAYER} has joined! {ONLINE} online'\ncustom-quit-message: '{USERNAME} left'");
        let alice = j(&mut t, "Alice");
        let bob = t.join("Bob");
        assert!(t.said(alice).contains(&"Bob has joined! 2 online".to_string()));
        assert!(t.said(bob).contains(&"Bob has joined! 2 online".to_string()));
        t.quit(bob);
        assert_eq!(t.said(alice), ["Bob left"]);
        assert!(join_messages_replaced(&t.ess));
    }

    #[test]
    fn silent_join_and_vanilla_fallback() {
        let mut t = srv("allow-silent-join-quit: true");
        t.permissions("groups:\n  default:\n    default: true\nusers:\n  carl:\n    permissions: [essentials.silentjoin, essentials.silentquit]\n");
        let alice = j(&mut t, "Alice");
        let _bob = t.join("Bob");
        assert!(t.said(alice).contains(&"Bob joined the game".to_string()));
        let carl = t.join("Carl");
        assert!(t.said(alice).is_empty());
        t.quit(carl);
        assert!(t.said(alice).is_empty());
        let bob = fake_uuid("Bob");
        t.quit(bob);
        assert_eq!(t.said(alice), ["Bob left the game"]);
    }

    #[test]
    fn vanilla_join_messages_stay_by_default() {
        let mut t = srv("");
        let alice = j(&mut t, "Alice");
        let _bob = t.join("Bob");
        assert!(!t.said(alice).iter().any(|l| l.contains("joined")));
        assert!(!join_messages_replaced(&t.ess));
    }

    // ----- command tree -----------------------------------------------------------------------------------

    #[test]
    fn command_tree_lists_every_label_as_greedy() {
        let t = srv("");
        let tree = command_tree(&t.ess);
        let msg = &tree["children"]["msg"];
        assert_eq!(msg["type"], "literal");
        assert_eq!(msg["executable"], true);
        assert_eq!(msg["children"]["args"]["parser"], "brigadier:string");
        assert_eq!(msg["children"]["args"]["properties"]["type"], "greedy");
        assert!(tree["children"]["tell"].is_object());
        assert!(tree["children"]["emsg"].is_object());
        assert!(tree["children"]["mail"].is_object());
    }

    // ----- utilities --------------------------------------------------------------------------------------

    #[test]
    fn date_diffs_parse_and_format() {
        let now = 1_700_000_000_000i64;
        assert_eq!(parse_date_diff(now, "10m", true), Some(now + 600_000));
        assert_eq!(parse_date_diff(now, "1h30m", true), Some(now + 5_400_000));
        assert_eq!(parse_date_diff(now, "2 days", true), Some(now + 2 * 86_400_000));
        assert_eq!(parse_date_diff(now, "spam", true), None);
        assert_eq!(format_date_diff(now, now + 600_000), "10 minutes");
        assert_eq!(format_date_diff(now, now + 60_000), "1 minute");
        assert_eq!(format_date_diff(now, now + 90_061_000), "1 day 1 hour 1 minute");
        assert_eq!(format_date_diff(now, now), "now");
    }

    #[test]
    fn regex_subset() {
        assert!(Regex::new("^[a-zA-Z_0-9§]+$").unwrap().is_match("Abc_9"));
        assert!(!Regex::new("^[a-zA-Z_0-9§]+$").unwrap().is_match("Ab!c"));
        assert!(Regex::new("^Dinner").unwrap().find_in("Dinnerbone"));
        assert!(Regex::new("(?i)notch|jeb").unwrap().find_in("xJEBx"));
        assert!(Regex::new("a{2,3}b").unwrap().is_match("aaab"));
        assert!(!Regex::new("a{2,3}b").unwrap().is_match("ab"));
        assert_eq!(block_url("see google.com and www.x.org/p"), "see google com and www.x org/p");
    }
}
