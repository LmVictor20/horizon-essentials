// horizon-essentials area `social` (pure logic, standalone-tested): Chat & social: msg, r, mail, me, ignore, socialspy, helpop, broadcast*, the Chat module (formats, local/shout/question), afk, nick, realname, motd, rules, info, customtext, help, join/quit messages, mute.
// Ported from EssentialsX (GPL-3.0): commands/Command{afk,broadcast,broadcastworld,customtext,help,helpop,ignore,
// info,mail,me,motd,msg,msgtoggle,mute,nick,r,realname,rtoggle,rules,socialspy}.java, EssentialsChat (chat
// handler, toggleshout), messaging/SimpleMessageRecipient, User/UserData (nick, afk, mute, mail, ignore),
// EssentialsPlayerListener (join/quit/chat/move/command), textreader/* and utils/{FormatUtil,DateUtil}.
//
// Text inside this module is Bukkit's: legacy `§` strings (display names, chat formats, text files). They become
// MiniMessage only when sent (`to_mini`), as EssentialsX's AdventureFacet does.

use crate::game::ess_core::*;
use crate::game::ess_meta;
use crate::game::ess_text::{self, escape_tags, legacy_to_mini};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

// ===== State ======================================================================================

/// What EssentialsX keeps in memory per player (User's transient fields) and globally.
#[derive(Default)]
pub struct Social {
    /// IMessageRecipient.replyRecipient.
    reply: HashMap<Sender, Sender>,
    /// SimpleMessageRecipient.lastMessageMs.
    last_message: HashMap<Sender, i64>,
    /// User.ignoreMsg (/msgtoggle).
    ignore_msg: HashSet<Uuid>,
    /// User.toggleShout when shouting does not persist.
    shout: HashMap<Uuid, bool>,
    /// User.lastActivity.
    activity: HashMap<Uuid, i64>,
    afk_pos: HashMap<Uuid, Loc>,
    afk_message: HashMap<Uuid, String>,
    afk_since: HashMap<Uuid, i64>,
    /// User.lastNotifiedAboutMailsMs.
    mail_notified: HashMap<Uuid, i64>,
    /// The display and tab names last given to the host (legacy text).
    shown_display: HashMap<Uuid, String>,
    shown_tab: HashMap<Uuid, Option<String>>,
    /// Commandmail's static (timestamp, mailsPerMinute).
    mail_minute: (i64, i64),
    /// EssentialsPlayerListener.pendingMotdTasks.
    motd_tasks: HashMap<Uuid, u64>,
}

fn st(ess: &mut Ess) -> &mut Social {
    ess.state::<Social>()
}

pub fn register(ess: &mut Ess) {
    let cmds: &[(&'static str, CmdFn, &'static [(&'static str, &'static str)])] = &[
        ("afk", cmd_afk, &[("/<command> [message]", "afkCommandUsage1Description"), ("/<command> <player> [message]", "afkCommandUsage2Description")]),
        ("broadcast", cmd_broadcast, &[("/<command> <message>", "broadcastCommandUsage1Description")]),
        ("broadcastworld", cmd_broadcastworld, &[("/<command> <world> <msg>", "broadcastworldCommandUsage1Description")]),
        ("customtext", cmd_customtext, &[]),
        ("help", cmd_help, &[]),
        ("helpop", cmd_helpop, &[("/<command> <message>", "helpopCommandUsage1Description")]),
        ("ignore", cmd_ignore, &[("/<command> <player>", "ignoreCommandUsage1Description")]),
        ("info", cmd_info, &[]),
        (
            "mail",
            cmd_mail,
            &[
                ("/<command> read [page]", "mailCommandUsage1Description"),
                ("/<command> clear [number]", "mailCommandUsage2Description"),
                ("/<command> clear <player> [number]", "mailCommandUsage3Description"),
                ("/<command> clearall", "mailCommandUsage4Description"),
                ("/<command> send <player> <message>", "mailCommandUsage5Description"),
                ("/<command> sendall <message>", "mailCommandUsage6Description"),
                ("/<command> sendtemp <player> <expire time> <message>", "mailCommandUsage7Description"),
                ("/<command> sendtempall <expire time> <message>", "mailCommandUsage8Description"),
            ],
        ),
        ("me", cmd_me, &[("/<command> <description>", "meCommandUsage1Description")]),
        ("motd", cmd_motd, &[]),
        ("msg", cmd_msg, &[("/<command> <to> <message>", "msgCommandUsage1Description")]),
        ("msgtoggle", cmd_msgtoggle, &[("/<command> [player]", "msgtoggleCommandUsage1Description")]),
        ("mute", cmd_mute, &[("/<command> <player>", "muteCommandUsage1Description"), ("/<command> <player> <datediff> [reason]", "muteCommandUsage2Description")]),
        (
            "nick",
            cmd_nick,
            &[
                ("/<command> <nickname>", "nickCommandUsage1Description"),
                ("/<command> off", "nickCommandUsage2Description"),
                ("/<command> <player> <nickname>", "nickCommandUsage3Description"),
                ("/<command> <player> off", "nickCommandUsage4Description"),
            ],
        ),
        ("r", cmd_r, &[("/<command> <message>", "rCommandUsage1Description")]),
        ("realname", cmd_realname, &[("/<command> <nickname>", "realnameCommandUsage1Description")]),
        ("rtoggle", cmd_rtoggle, &[]),
        ("rules", cmd_rules, &[]),
        ("socialspy", cmd_socialspy, &[("/<command> [player]", "socialspyCommandUsage1Description")]),
        ("toggleshout", cmd_toggleshout, &[("/<command> [player]", "toggleshoutCommandUsage1Description")]),
    ];
    for (name, run, usages) in cmds {
        ess.register(Command { name, run: *run, tab: None, usages });
    }
    ess.listeners.join.push(on_join);
    ess.listeners.quit.push(on_quit);
    ess.listeners.tick.push(on_tick);
    ess.listeners.chat.push(on_chat);
    ess.listeners.command.push(on_command);
    ess.listeners.moved.push(on_move);
    ess.listeners.interact.push(on_interact);
}

// ===== Small helpers ==============================================================================

fn cfg_str(ess: &Ess, key: &str, default: &str) -> String {
    ess.config_str(key).unwrap_or_else(|| default.to_string())
}

fn cmd_disabled(ess: &Ess, name: &str) -> bool {
    ess.config_list("disabled-commands").iter().any(|d| d.eq_ignore_ascii_case(name))
}

fn is_online(host: &mut dyn Host, u: Uuid) -> bool {
    host.online().contains(&u)
}

/// The player's name: online, else the last one Essentials saw.
fn name_of(ess: &mut Ess, host: &mut dyn Host, u: Uuid) -> String {
    host.name(u).or_else(|| ess.users.get(u).and_then(|x| x.name())).unwrap_or_else(|| uuid_string(u))
}

fn user_bool(ess: &mut Ess, u: Uuid, key: &str) -> bool {
    ess.users.get(u).is_some_and(|x| x.bool(key))
}

fn user_str(ess: &mut Ess, u: Uuid, key: &str) -> Option<String> {
    ess.users.get(u).and_then(|x| x.get(key).cloned()).and_then(|v| match v {
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

/// A legacy (`§`) string as MiniMessage text that can't inject tags.
pub fn to_mini(legacy: &str) -> String {
    legacy_to_mini(&escape_tags(legacy), false)
}

fn send_legacy(ess: &Ess, host: &mut dyn Host, to: Sender, legacy: &str) {
    ess.send_mini(host, to, &to_mini(legacy));
}

/// User.sendTl: only reaches players who are online.
fn tell(ess: &mut Ess, host: &mut dyn Host, u: Uuid, key: &str, args: &[Arg]) {
    if is_online(host, u) {
        ess.send_tl(host, Sender::Player(u), key, args);
    }
}

/// I18n.capitalCase.
fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// MessageFormat's rendering of a double ("0.5", "5").
fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 {
        ess_text::group_digits(&format!("{}", x as i64))
    } else {
        let s = format!("{x:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Java's Double.toString for simple values ("1.0", "0.5").
fn java_double(x: f64) -> String {
    if x.fract() == 0.0 { format!("{x:.1}") } else { format!("{x}") }
}

/// Bukkit's world name for a dimension (world, world_nether, world_the_end).
pub fn world_name(dim: &str) -> String {
    match dim {
        "minecraft:overworld" => "world".into(),
        "minecraft:the_nether" => "world_nether".into(),
        "minecraft:the_end" => "world_the_end".into(),
        other => other.rsplit(':').next().unwrap_or(other).to_string(),
    }
}

/// Essentials.getWorld: a world by Bukkit name or dimension id.
fn find_world(host: &mut dyn Host, name: &str) -> Option<String> {
    host.worlds().into_iter().find(|w| world_name(w).eq_ignore_ascii_case(name) || w.eq_ignore_ascii_case(name) || *w == world_id(&name.to_lowercase()))
}

/// Server.matchPlayer: an exact name, else every online name containing the text.
fn match_players(host: &mut dyn Host, partial: &str) -> Vec<Uuid> {
    let p = partial.to_lowercase();
    let mut out = Vec::new();
    for u in host.online() {
        let Some(n) = host.name(u) else { continue };
        if n.eq_ignore_ascii_case(partial) {
            return vec![u];
        }
        if n.to_lowercase().contains(&p) {
            out.push(u);
        }
    }
    out
}

/// A player the sender may not see (vanished, and the sender can't interact with vanished players).
fn hidden_from(ess: &mut Ess, host: &mut dyn Host, sender: Sender, u: Uuid) -> bool {
    sender.player().is_some_and(|s| s != u && ess.is_vanished(u) && !ess.has(host, sender, "essentials.vanish.interact"))
}

// ===== Permissions provider: groups, prefixes, suffixes ===========================================

/// The player's groups with inheritance, primary first.
fn group_chain(ess: &mut Ess, host: &mut dyn Host, u: Uuid) -> Vec<String> {
    let name = name_of(ess, host, u);
    let mut queue = ess.perms.groups_of(u, &name);
    let mut seen = Vec::new();
    while !queue.is_empty() {
        let g = queue.remove(0);
        if seen.contains(&g) {
            continue;
        }
        if let Some(group) = ess.perms.groups.get(&g) {
            queue.extend(group.inherits.iter().cloned());
        }
        seen.push(g);
    }
    seen
}

/// PermissionsHandler.getPrefix/getSuffix (the first group in the chain that sets one), `&` codes replaced.
fn perm_affix(ess: &mut Ess, host: &mut dyn Host, u: Uuid, suffix: bool) -> String {
    for g in group_chain(ess, host, u) {
        if let Some(group) = ess.perms.groups.get(&g) {
            let v = if suffix { &group.suffix } else { &group.prefix };
            if !v.is_empty() {
                return replace_format(v);
            }
        }
    }
    String::new()
}

fn is_perm_set(ess: &mut Ess, host: &mut dyn Host, who: Sender, node: &str) -> bool {
    match who {
        Sender::Console => false,
        Sender::Player(u) => {
            let name = name_of(ess, host, u);
            ess.perms.configured(u, &name, &node.to_lowercase()).is_some()
        }
    }
}

// ===== FormatUtil =================================================================================

const COLOR_CODES: &str = "0123456789abcdef";
const ALL_CODES: &str = "0123456789abcdefklmnor";
/// (ChatColor name for `<base>.<name>` nodes, code).
const CHAT_COLORS: [(&str, char); 22] = [
    ("black", '0'), ("dark_blue", '1'), ("dark_green", '2'), ("dark_aqua", '3'), ("dark_red", '4'), ("dark_purple", '5'),
    ("gold", '6'), ("gray", '7'), ("dark_gray", '8'), ("blue", '9'), ("green", 'a'), ("aqua", 'b'), ("red", 'c'),
    ("light_purple", 'd'), ("yellow", 'e'), ("white", 'f'), ("obfuscated", 'k'), ("bold", 'l'), ("strikethrough", 'm'),
    ("underline", 'n'), ("italic", 'o'), ("reset", 'r'),
];

/// FormatUtil.replaceFormat: every `&` code (and `&#rrggbb`) to `§`; `&&c` stays `&c`.
pub fn replace_format(s: &str) -> String {
    replace_color(s, ALL_CODES, true)
}

fn replace_color(s: &str, supported: &str, rgb: bool) -> String {
    let c: Vec<char> = s.chars().collect();
    let is_code = |ch: char| ALL_CODES.contains(ch.to_ascii_lowercase());
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '&' && c.get(i + 1) == Some(&'&') && c.get(i + 2).is_some_and(|x| is_code(*x)) {
            out.push('&');
            out.push(c[i + 2]);
            i += 3;
            continue;
        }
        if c[i] == '&' && c.get(i + 1).is_some_and(|x| is_code(*x)) {
            let code = c[i + 1];
            out.push(if supported.contains(code.to_ascii_lowercase()) { '§' } else { '&' });
            out.push(code);
            i += 2;
            continue;
        }
        out.push(c[i]);
        i += 1;
    }
    if !rgb {
        return out;
    }
    let c: Vec<char> = out.chars().collect();
    let hex_at = |i: usize| i + 7 <= c.len() && c[i] == '#' && c[i + 1..i + 7].iter().all(|x| x.is_ascii_hexdigit());
    let mut res = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '&' && c.get(i + 1) == Some(&'&') && hex_at(i + 2) {
            res.push('&');
            res.extend(&c[i + 2..i + 9]);
            i += 9;
            continue;
        }
        if c[i] == '&' && hex_at(i + 1) {
            res.push_str("§x");
            for h in &c[i + 2..i + 8] {
                res.push('§');
                res.push(*h);
            }
            i += 8;
            continue;
        }
        res.push(c[i]);
        i += 1;
    }
    res
}

/// Removes `§` codes whose code is in `strip` (FormatUtil.stripColor(input, set)).
fn strip_color(s: &str, strip: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '§' {
            let mut j = i;
            while j < c.len() && c[j] == '§' {
                j += 1;
            }
            if let Some(code) = c.get(j).map(|x| x.to_ascii_lowercase()).filter(|x| ALL_CODES.contains(*x)) {
                if !strip.contains(code) {
                    out.extend(&c[i..=j]);
                }
                i = j + 1;
                continue;
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// FormatUtil.stripFormat (ChatColor.stripColor): every `§` code.
pub fn strip_format(s: &str) -> String {
    ess_text::strip_legacy(s, false)
}

/// FormatUtil.stripEssentialsFormat: `&` codes and `&#rrggbb`.
fn strip_essentials_format(s: &str) -> String {
    let mut out = String::new();
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '&' && c.get(i + 1).is_some_and(|x| ALL_CODES.contains(x.to_ascii_lowercase())) {
            i += 2;
            continue;
        }
        if c[i] == '&' && c.get(i + 1) == Some(&'#') && i + 8 <= c.len() && c[i + 2..i + 8].iter().all(|x| x.is_ascii_hexdigit()) {
            i += 8;
            continue;
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// FormatUtil.unformatString with every code: `§` back to `&`.
fn unformat_string(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '§' && c.get(i + 1) == Some(&'x') && i + 14 <= c.len() && (0..6).all(|k| c[i + 2 + k * 2] == '§') {
            out.push_str("&#");
            for k in 0..6 {
                out.push(c[i + 3 + k * 2]);
            }
            i += 14;
            continue;
        }
        if c[i] == '§' && c.get(i + 1).is_some_and(|x| ALL_CODES.contains(x.to_ascii_lowercase())) {
            out.push('&');
            out.push(c[i + 1].to_ascii_lowercase());
            i += 2;
            continue;
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// FormatUtil.getSupported: the codes `<base>.color/.format/.magic` and per-color nodes allow, and `.rgb`.
fn supported_codes(ess: &mut Ess, host: &mut dyn Host, who: Sender, base: &str) -> (String, bool) {
    let mut sup: HashSet<char> = HashSet::new();
    if ess.has(host, who, &format!("{base}.color")) {
        sup.extend(COLOR_CODES.chars());
    }
    if ess.has(host, who, &format!("{base}.format")) {
        sup.extend("lmnor".chars());
    }
    if ess.has(host, who, &format!("{base}.magic")) {
        sup.insert('k');
    }
    for (name, code) in CHAT_COLORS {
        let node = format!("{base}.{name}");
        if !is_perm_set(ess, host, who, &node) {
            continue;
        }
        if ess.has(host, who, &node) {
            sup.insert(code);
        } else {
            sup.remove(&code);
        }
    }
    let rgb = ess.has(host, who, &format!("{base}.rgb"));
    (ALL_CODES.chars().filter(|c| sup.contains(c)).collect(), rgb)
}

/// FormatUtil.formatString: codes the sender may use become colors, others are stripped or left as typed.
fn format_string(ess: &mut Ess, host: &mut dyn Host, who: Sender, base: &str, msg: &str) -> String {
    if who == Sender::Console {
        return replace_format(msg);
    }
    let (sup, rgb) = supported_codes(ess, host, who, base);
    let strip: String = ALL_CODES.chars().filter(|c| !sup.contains(*c)).collect();
    let mut m = msg.to_string();
    if !strip.is_empty() {
        m = strip_color(&m, &strip);
    }
    if !sup.is_empty() || rgb {
        m = replace_color(&m, &sup, rgb);
    }
    m
}

/// FormatUtil.formatMessage: formatString, then URLs broken up without `<base>.url`.
fn format_message(ess: &mut Ess, host: &mut dyn Host, who: Sender, base: &str, msg: &str) -> String {
    let m = format_string(ess, host, who, base, msg);
    if who != Sender::Console && !ess.has(host, who, &format!("{base}.url")) { block_url(&m) } else { m }
}

/// FormatUtil.blockURL: `google.com` -> `google com`, until no URL is left.
pub fn block_url(s: &str) -> String {
    let re = Regex::new(r"((?:(?:https?)://)?[\w-_\.]{2,})\.([a-zA-Z]{2,}(?:/\S+)?)").expect("url pattern");
    let mut text = re.replace_all(s, |c| format!("{} {}", c[1], c[2]));
    while re.find_in(&text) {
        text = re.replace_all(&text, |c| format!("{} {}", c[1], c[2]));
    }
    text
}

/// StringUtil.sanitizeString: drops control characters.
fn sanitize_string(s: &str) -> String {
    s.chars().filter(|c| matches!(*c, '\t' | '\n' | '\r') || (' '..='~').contains(c) || *c == '\u{85}' || ('\u{a0}'..='\u{d7ff}').contains(c) || ('\u{e000}'..='\u{fffc}').contains(c)).collect()
}

/// StringUtil.sanitizeFileName.
fn sanitize_file_name(s: &str) -> String {
    s.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { c } else { '_' }).collect()
}

// ===== Names: nicknames, display names, tab names =================================================

fn nickname(ess: &mut Ess, u: Uuid) -> Option<String> {
    user_str(ess, u, "nickname").filter(|n| !n.is_empty())
}

/// Settings.getOperatorColor.
fn op_color(ess: &Ess) -> Option<String> {
    let Some(c) = ess.config_str("ops-name-color") else { return Some("§c".into()) };
    if c.eq_ignore_ascii_case("none") || c.is_empty() {
        return None;
    }
    let hex = c.trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|x| x.is_ascii_hexdigit()) {
        return Some(format!("§x{}", hex.chars().map(|h| format!("§{h}")).collect::<String>()));
    }
    if let Some(i) = ess_text::COLOR_NAMES.iter().position(|n| n.eq_ignore_ascii_case(&c)) {
        return Some(format!("§{}", COLOR_CODES.as_bytes()[i] as char));
    }
    let ch = c.chars().next()?.to_ascii_lowercase();
    (c.chars().count() == 1 && ALL_CODES.contains(ch)).then(|| format!("§{ch}"))
}

/// Settings.addPrefixSuffix: true with EssentialsChat (always here) unless configured.
fn add_prefix_suffix(ess: &Ess) -> bool {
    ess.config_bool("add-prefix-suffix", true)
}

/// User.getNick(withPrefix, withSuffix).
fn get_nick(ess: &mut Ess, host: &mut dyn Host, u: Uuid, with_prefix: bool, with_suffix: bool) -> String {
    let name = name_of(ess, host, u);
    let nick = nickname(ess, u);
    let mut prefix = String::new();
    let mut suffix = String::new();
    let nickname = match nick {
        Some(n) if !cmd_disabled(ess, "nick") && n != name => {
            if n.eq_ignore_ascii_case(&name) {
                n
            } else {
                suffix = "§r".into();
                if ess.has(host, Sender::Player(u), "essentials.nick.hideprefix") { n } else { replace_format(&cfg_str(ess, "nickname-prefix", "~")) + &n }
            }
        }
        _ => name.clone(),
    };
    if host.op_level(u) > 0 {
        if let Some(c) = op_color(ess) {
            prefix.insert_str(0, &c);
            suffix = "§r".into();
        }
    }
    if add_prefix_suffix(ess) {
        if with_prefix || !ess.config_bool("disablePrefix", false) {
            let p = perm_affix(ess, host, u, false);
            prefix.insert_str(0, &p);
            suffix = "§r".into();
        }
        if with_suffix || !ess.config_bool("disableSuffix", false) {
            let s = perm_affix(ess, host, u, true);
            suffix = (s + "§r").replace("§f§r", "§r").replace("§r§r", "§r");
        }
    }
    let mut out = prefix + &nickname + &suffix;
    if out.ends_with('§') {
        out.pop();
    }
    out
}

/// User.getFormattedNickname.
fn formatted_nickname(ess: &mut Ess, u: Uuid) -> Option<String> {
    let n = nickname(ess, u)?;
    Some(replace_format(&(cfg_str(ess, "nickname-prefix", "~") + &n)))
}

/// User.getDisplayName (legacy text).
pub fn display_name(ess: &mut Ess, host: &mut dyn Host, u: Uuid) -> String {
    if ess.config_bool("hide-displayname-in-vanish", true) && ess.is_vanished(u) {
        return name_of(ess, host, u);
    }
    if ess.config_bool("change-displayname", true) { get_nick(ess, host, u, true, true) } else { name_of(ess, host, u) }
}

fn display_of(ess: &mut Ess, host: &mut dyn Host, who: Sender) -> String {
    match who {
        Sender::Console => ess.tl("consoleName", &[]),
        Sender::Player(u) => display_name(ess, host, u),
    }
}

fn set_tab(ess: &mut Ess, host: &mut dyn Host, u: Uuid, tab: Option<String>) {
    if st(ess).shown_tab.get(&u) == Some(&tab) {
        return;
    }
    let spans = tab.as_deref().map(|t| ess.render(&to_mini(t)));
    host.set_tab_name(u, spans.as_deref());
    st(ess).shown_tab.insert(u, tab);
}

/// User.setDisplayNick: the display name (and with change-playerlist, the tab name) given to the server.
fn set_display_nick(ess: &mut Ess, host: &mut dyn Host, u: Uuid) {
    if !is_online(host, u) || !ess.config_bool("change-displayname", true) {
        return;
    }
    let d = get_nick(ess, host, u, true, true);
    if st(ess).shown_display.get(&u) != Some(&d) {
        let spans = ess.render(&to_mini(&d));
        host.set_display_name(u, Some(&spans));
        st(ess).shown_display.insert(u, d);
    }
    if is_afk(ess, u) {
        update_afk_list_name(ess, host, u);
    } else if ess.config_bool("change-playerlist", false) {
        let (p, s) = (ess.config_bool("add-prefix-in-playerlist", false), ess.config_bool("add-suffix-in-playerlist", false));
        let tab = get_nick(ess, host, u, p, s);
        set_tab(ess, host, u, Some(tab));
    }
}

// ===== Ignore, mute ===============================================================================

fn ignore_list(ess: &mut Ess, u: Uuid) -> Vec<String> {
    match ess.users.get(u).and_then(|x| x.get("ignore").cloned()) {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

/// UserData.isIgnoredPlayer: `by` ignores `who` (unless `who` has essentials.chat.ignoreexempt).
fn ignores(ess: &mut Ess, host: &mut dyn Host, by: Uuid, who: Uuid) -> bool {
    ignore_list(ess, by).contains(&uuid_string(who)) && !ess.has(host, Sender::Player(who), "essentials.chat.ignoreexempt")
}

/// For other areas (tpa requests): whether `by` ignores `who`.
pub fn ignores_player(ess: &mut Ess, host: &mut dyn Host, by: Uuid, who: Uuid) -> bool {
    ignores(ess, host, by, who)
}

pub fn is_muted(ess: &mut Ess, u: Uuid) -> bool {
    user_bool(ess, u, "muted")
}

fn mute_timeout(ess: &mut Ess, u: Uuid) -> i64 {
    ess.users.get(u).and_then(|x| x.i64("timestamps.mute")).unwrap_or(0)
}

fn mute_reason(ess: &mut Ess, u: Uuid) -> Option<String> {
    user_str(ess, u, "mute-reason").filter(|r| !r.is_empty())
}

/// The voiceSilenced* message for a muted player.
fn muted_tl(ess: &mut Ess, host: &mut dyn Host, u: Uuid) -> (&'static str, Vec<Arg>) {
    let timeout = mute_timeout(ess, u);
    let diff = (timeout > 0).then(|| format_date_diff(host.now_millis(), timeout));
    match (diff, mute_reason(ess, u)) {
        (None, None) => ("voiceSilenced", vec![]),
        (None, Some(r)) => ("voiceSilencedReason", vec![Arg::Plain(r)]),
        (Some(d), None) => ("voiceSilencedTime", vec![Arg::Plain(d)]),
        (Some(d), Some(r)) => ("voiceSilencedReasonTime", vec![Arg::Plain(d), Arg::Plain(r)]),
    }
}

/// User.notifyMuted.
fn notify_muted(ess: &mut Ess, host: &mut dyn Host, u: Uuid) {
    let (key, args) = muted_tl(ess, host, u);
    tell(ess, host, u, key, &args);
}

fn muted_error(ess: &mut Ess, host: &mut dyn Host, u: Uuid) -> CmdError {
    let (key, args) = muted_tl(ess, host, u);
    CmdError::Tl(key, args)
}

/// User.checkMuteTimeout.
fn check_mute_timeout(ess: &mut Ess, host: &mut dyn Host, u: Uuid, now: i64) {
    let timeout = mute_timeout(ess, u);
    if timeout > 0 && timeout < now && is_muted(ess, u) {
        let user = ess.users.user(u);
        user.set("timestamps.mute", json!(0));
        user.set("muted", json!(false));
        user.set("mute-reason", Value::Null);
        tell(ess, host, u, "canTalkAgain", &[]);
    }
}

/// Essentials.broadcastTl(sender, permission/exclusion, key, args): to online players only.
fn broadcast_tl(ess: &mut Ess, host: &mut dyn Host, sender: Option<Uuid>, perm: Option<&str>, except: Option<Uuid>, key: &str, args: &[Arg]) {
    if let Some(s) = sender {
        if ess.is_vanished(s) {
            return;
        }
    }
    for p in host.online() {
        if sender.is_some_and(|s| ignores(ess, host, p, s)) || Some(p) == except {
            continue;
        }
        if perm.is_some_and(|n| !ess.has(host, Sender::Player(p), n)) {
            continue;
        }
        ess.send_tl(host, Sender::Player(p), key, args);
    }
}

/// Server.broadcastMessage of a legacy line: every online player (but `except`) and the console.
fn broadcast_legacy(ess: &mut Ess, host: &mut dyn Host, line: &str, except: Option<Uuid>) {
    for p in host.online() {
        if Some(p) != except {
            send_legacy(ess, host, Sender::Player(p), line);
        }
    }
    send_legacy(ess, host, Sender::Console, line);
}

// ===== AFK ========================================================================================

pub fn is_afk(ess: &mut Ess, u: Uuid) -> bool {
    user_bool(ess, u, "afk")
}

fn update_afk_list_name(ess: &mut Ess, host: &mut dyn Host, u: Uuid) {
    let list = replace_format(&cfg_str(ess, "afk-list-name", "none"));
    if list.eq_ignore_ascii_case("none") {
        return;
    }
    if is_afk(ess, u) {
        let display = display_name(ess, host, u);
        let name = name_of(ess, host, u);
        set_tab(ess, host, u, Some(list.replace("{PLAYER}", &display).replace("{USERNAME}", &name)));
    } else {
        set_tab(ess, host, u, None);
        set_display_nick(ess, host, u);
    }
}

/// User.setAfk.
fn set_afk(ess: &mut Ess, host: &mut dyn Host, u: Uuid, set: bool) {
    let afk = is_afk(ess, u);
    let now = host.now_millis();
    if set && !afk {
        if let Some(at) = host.location(u) {
            st(ess).afk_pos.insert(u, at);
        }
        st(ess).afk_since.insert(u, now);
    } else if !set && afk {
        let s = st(ess);
        s.afk_pos.remove(&u);
        s.afk_message.remove(&u);
        s.afk_since.remove(&u);
    }
    ess.users.user(u).set("afk", json!(set));
    update_afk_list_name(ess, host, u);
}

/// User.updateActivity.
fn update_activity(ess: &mut Ess, host: &mut dyn Host, u: Uuid, broadcast: bool) {
    if is_afk(ess, u) {
        set_afk(ess, host, u, false);
        if broadcast && !ess.is_vanished(u) && !is_afk(ess, u) {
            set_display_nick(ess, host, u);
            let display = display_name(ess, host, u);
            if ess.config_bool("broadcast-afk-message", true) {
                broadcast_tl(ess, host, Some(u), None, Some(u), "userIsNotAway", &[Arg::Plain(display.clone())]);
            }
            tell(ess, host, u, "userIsNotAwaySelf", &[Arg::Plain(display)]);
        }
    }
    let now = host.now_millis();
    st(ess).activity.insert(u, now);
}

fn update_activity_on_interact(ess: &mut Ess, host: &mut dyn Host, u: Uuid, broadcast: bool) {
    if ess.config_bool("cancel-afk-on-interact", true) {
        update_activity(ess, host, u, broadcast);
    }
}

fn update_activity_on_move(ess: &mut Ess, host: &mut dyn Host, u: Uuid, broadcast: bool) {
    if ess.config_bool("cancel-afk-on-move", true) {
        update_activity(ess, host, u, broadcast);
    }
}

/// User.checkActivity (EssentialsTimer): auto-afk-timeout kick or commands, then auto-afk.
fn check_activity(ess: &mut Ess, host: &mut dyn Host, u: Uuid) {
    let now = host.now_millis();
    let Some(last) = st(ess).activity.get(&u).copied() else {
        st(ess).activity.insert(u, now);
        return;
    };
    if now - last <= 10_000 {
        return;
    }
    let timeout = ess.config_i64("auto-afk-timeout", -1);
    let who = Sender::Player(u);
    if timeout > 0 && last > 0 && last + timeout * 1000 < now && !ess.has(host, who, "essentials.kick.exempt") && !ess.has(host, who, "essentials.afk.kickexempt") {
        st(ess).activity.insert(u, 0);
        let kick_time = timeout as f64 / 60.0;
        let commands: Vec<String> = ess.config_list("afk-timeout-commands").into_iter().filter(|c| !c.is_empty()).collect();
        let name = name_of(ess, host, u);
        if commands.is_empty() {
            let reason = ess.tl("autoAfkKickReason", &[Arg::Plain(fmt_num(kick_time))]);
            let spans = ess.render(&reason);
            host.kick(u, &spans);
            let console = ess.tl("consoleName", &[]);
            for p in host.online() {
                if ess.has(host, Sender::Player(p), "essentials.kick.notify") {
                    ess.send_tl(host, Sender::Player(p), "playerKicked", &[Arg::Plain(console.clone()), Arg::Plain(name.clone()), Arg::Parsed(reason.clone())]);
                }
            }
            return;
        }
        for c in commands {
            host.console_command(&c.replace("{USERNAME}", &name).replace("{KICKTIME}", &java_double(kick_time)));
        }
    }
    let last = st(ess).activity.get(&u).copied().unwrap_or(0);
    let auto = ess.config_i64("auto-afk", 300);
    if !is_afk(ess, u) && auto > 0 && last + auto * 1000 < now && ess.has(host, who, "essentials.afk.auto") {
        set_afk(ess, host, u, true);
        if is_afk(ess, u) && !ess.is_vanished(u) {
            set_display_nick(ess, host, u);
            let display = display_name(ess, host, u);
            if ess.config_bool("broadcast-afk-message", true) {
                broadcast_tl(ess, host, Some(u), None, Some(u), "userIsAway", &[Arg::Plain(display.clone())]);
            }
            tell(ess, host, u, "userIsAwaySelf", &[Arg::Plain(display)]);
        }
    }
}

// ===== Listeners ==================================================================================

fn hide_join_quit(ess: &Ess, host: &mut dyn Host) -> bool {
    let above = ess.config_i64("hide-join-quit-messages-above", -1);
    above >= 0 && host.online().len() as i64 > above
}

/// Whether Essentials replaces the vanilla join line (EssentialsPlayerListener's legacy join flow nulls it then);
/// the glue then suppresses vanilla's and Essentials sends its own.
pub fn join_messages_replaced(ess: &Ess) -> bool {
    !cfg_str(ess, "custom-join-message", "none").eq_ignore_ascii_case("none") || ess.config_bool("allow-silent-join-quit", false) || ess.config_i64("hide-join-quit-messages-above", -1) >= 0
}

/// The same for the quit line.
pub fn quit_messages_replaced(ess: &Ess) -> bool {
    !cfg_str(ess, "custom-quit-message", "none").eq_ignore_ascii_case("none") || ess.config_bool("allow-silent-join-quit", false) || ess.config_i64("hide-join-quit-messages-above", -1) >= 0
}

fn join_placeholders(ess: &mut Ess, host: &mut dyn Host, u: Uuid, template: &str, online: usize) -> String {
    let display = display_name(ess, host, u);
    let name = name_of(ess, host, u);
    let unique = ess.users.all().len();
    let now = host.now_millis();
    let uptime = format_date_diff(now, now - host.stats().uptime_ms);
    let prefix = perm_affix(ess, host, u, false);
    let suffix = perm_affix(ess, host, u, true);
    replace_format(template)
        .replace("{PLAYER}", &display)
        .replace("{USERNAME}", &name)
        .replace("{UNIQUE}", &ess_text::group_digits(&unique.to_string()))
        .replace("{ONLINE}", &ess_text::group_digits(&online.to_string()))
        .replace("{UPTIME}", &uptime)
        .replace("{PREFIX}", &prefix)
        .replace("{SUFFIX}", &suffix)
        .replace("{OLDUSERNAME}", "")
}

/// EssentialsPlayerListener: delayedJoin + joinFlow.
fn on_join(ess: &mut Ess, host: &mut dyn Host, u: Uuid, _first: bool) {
    let now = host.now_millis();
    check_mute_timeout(ess, host, u, now);
    update_activity(ess, host, u, false);
    st(ess).shown_display.remove(&u);
    st(ess).shown_tab.remove(&u);
    set_display_nick(ess, host, u);

    let who = Sender::Player(u);
    let silent_ok = ess.config_bool("allow-silent-join-quit", false);
    let message = if silent_ok && (ess.has(host, who, "essentials.silentjoin") || ess.has(host, who, "essentials.silentjoin.vanish")) {
        if ess.has(host, who, "essentials.silentjoin.vanish") {
            ess.users.user(u).set("vanished", json!(true));
            host.set_hidden(u, true, &[]);
        }
        None
    } else if hide_join_quit(ess, host) {
        None
    } else {
        let custom = cfg_str(ess, "custom-join-message", "none");
        if !custom.eq_ignore_ascii_case("none") {
            let online = host.online().len();
            Some(join_placeholders(ess, host, u, &custom, online)).filter(|m| !m.is_empty())
        } else {
            Some(format!("§e{} joined the game", name_of(ess, host, u)))
        }
    };
    if join_messages_replaced(ess) {
        if let Some(m) = message {
            broadcast_legacy(ess, host, &m, None);
        }
    }

    let delay = ess.config_i64("delay-motd", 0);
    if delay >= 0 {
        let ticks = (delay / 50) as u64;
        if ticks > 0 {
            let id = ess.scheduler.later(ticks, Box::new(move |ess, host| motd_flow(ess, host, u)));
            st(ess).motd_tasks.insert(u, id);
        } else {
            motd_flow(ess, host, u);
        }
    }

    if !cmd_disabled(ess, "mail") && ess.has(host, who, "essentials.mail") {
        if unread_mail(ess, host, u) == 0 {
            if ess.config_bool("notify-no-new-mail", true) {
                tell(ess, host, u, "noNewMail", &[]);
            }
        } else {
            notify_mail(ess, host, u);
        }
    }

    if user_bool(ess, u, "socialspy") && !ess.has(host, who, "essentials.socialspy") {
        ess.users.user(u).set("socialspy", json!(false));
        let name = name_of(ess, host, u);
        ess.send_mini(host, Sender::Console, &escape_tags(&format!("Set socialspy to false for {name} because they had it enabled without permission.")));
    }
}

fn motd_flow(ess: &mut Ess, host: &mut dyn Host, u: Uuid) {
    st(ess).motd_tasks.remove(&u);
    if cmd_disabled(ess, "motd") || !is_online(host, u) {
        return;
    }
    let who = Sender::Player(u);
    let text = match read_text(ess, host, who, "motd", true) {
        Ok(t) => t,
        Err(CmdError::Msg(m)) => {
            ess.send_mini(host, Sender::Console, &escape_tags(&m));
            return;
        }
        Err(_) => return,
    };
    if !text.lines.is_empty() && ess.has(host, who, "essentials.motd") {
        let text = keyword_text(ess, host, who, text);
        show_page(ess, host, who, &text, Some("1"), None, Some("motd"), true);
    }
}

/// EssentialsPlayerListener.onPlayerQuit.
fn on_quit(ess: &mut Ess, host: &mut dyn Host, u: Uuid) {
    if let Some(id) = st(ess).motd_tasks.remove(&u) {
        ess.scheduler.cancel(id);
    }
    let silent = ess.config_bool("allow-silent-join-quit", false) && ess.has(host, Sender::Player(u), "essentials.silentquit");
    let message = if hide_join_quit(ess, host) || silent {
        None
    } else {
        let custom = cfg_str(ess, "custom-quit-message", "none");
        if !custom.eq_ignore_ascii_case("none") {
            let online = host.online().iter().filter(|p| **p != u).count();
            Some(join_placeholders(ess, host, u, &custom, online)).filter(|m| !m.is_empty())
        } else {
            Some(format!("§e{} left the game", name_of(ess, host, u)))
        }
    };
    if quit_messages_replaced(ess) {
        if let Some(m) = message {
            broadcast_legacy(ess, host, &m, Some(u));
        }
    }
    update_activity(ess, host, u, false);
    let s = st(ess);
    s.shown_display.remove(&u);
    s.shown_tab.remove(&u);
    s.activity.remove(&u);
}

/// EssentialsTimer every 50 ticks: activity (auto-afk, timeout kick) and mute expiry.
fn on_tick(ess: &mut Ess, host: &mut dyn Host) {
    if ess.scheduler.tick % 50 != 0 {
        return;
    }
    let now = host.now_millis();
    for u in host.online() {
        check_activity(ess, host, u);
        check_mute_timeout(ess, host, u, now);
    }
}

/// EssentialsPlayerListener.onPlayerMove: AFK freeze, cancel-afk-on-move.
fn on_move(ess: &mut Ess, host: &mut dyn Host, u: Uuid, from: &Loc, to: &Loc) {
    if from.block() == to.block() && from.world == to.world {
        return;
    }
    let cancel = ess.config_bool("cancel-afk-on-move", true);
    let freeze = ess.config_bool("freeze-afk-players", false);
    if !cancel && !freeze {
        return;
    }
    if is_afk(ess, u) && freeze {
        if to.y >= f64::from(from.block()[1] + 1) {
            update_activity_on_move(ess, host, u, true);
            return;
        }
        let mut back = to.clone();
        back.world = from.world.clone();
        back.x = from.x;
        back.y = from.y;
        back.z = from.z;
        host.teleport(u, &back);
        ess.last_loc.insert(u, back);
        return;
    }
    let far = match st(ess).afk_pos.get(&u) {
        None => true,
        Some(p) => p.world != to.world || (p.x - to.x).powi(2) + (p.y - to.y).powi(2) + (p.z - to.z).powi(2) > 9.0,
    };
    if far {
        update_activity_on_move(ess, host, u, true);
    }
}

fn on_interact(ess: &mut Ess, host: &mut dyn Host, u: Uuid, _block: Option<(&str, [i32; 3], &str)>, _held: Option<&Item>) -> Allow {
    update_activity_on_interact(ess, host, u, true);
    Allow::Yes
}

/// EssentialsPlayerListener.handlePlayerCommandPreprocess: socialspy-commands, mute-commands, AFK activity.
fn on_command(ess: &mut Ess, host: &mut dyn Host, sender: Sender, line: &str) -> Allow {
    let Sender::Player(u) = sender else { return Allow::Yes };
    let line = line.trim_start_matches('/');
    let label = line.split_whitespace().next().unwrap_or("").to_lowercase();
    let label = label.rsplit(':').next().unwrap_or("").to_string();
    let name = ess.command_for(&label);
    let full = format!("/{line}");
    let spy_cmds: Vec<String> = ess.config_list("socialspy-commands").iter().map(|c| c.to_lowercase()).collect();
    if (spy_cmds.contains(&label) || spy_cmds.iter().any(|c| c == "*")) && !matches!(name, Some("msg") | Some("r")) && !ess.has(host, sender, "essentials.chat.spy.exempt") {
        let pname = if ess.config_bool("socialspy-uses-displaynames", true) { display_name(ess, host, u) } else { name_of(ess, host, u) };
        let muted = is_muted(ess, u) && ess.config_bool("socialspy-listen-muted-players", true);
        let prefix = ess.tl(if muted { "socialSpyMutedPrefix" } else { "socialSpyPrefix" }, &[]);
        let body = ess.tl("socialSpyCmdFormat", &[Arg::Plain(pname), Arg::Plain(full.clone())]);
        for s in host.online() {
            if s != u && user_bool(ess, s, "socialspy") {
                ess.send_mini(host, Sender::Player(s), &format!("{prefix}{body}"));
            }
        }
    }
    let mute_cmds: Vec<String> = ess.config_list("mute-commands").iter().map(|c| c.to_lowercase()).collect();
    if is_muted(ess, u) && (mute_cmds.contains(&label) || mute_cmds.iter().any(|c| c == "*")) {
        notify_muted(ess, host, u);
        let pname = name_of(ess, host, u);
        ess.send_tl(host, Sender::Console, "mutedUserSpeaks", &[Arg::Plain(pname), Arg::Plain(full)]);
        return Allow::No;
    }
    let (update, broadcast) = match name {
        Some("afk") => (false, false),
        Some("vanish") => (true, false),
        _ => (true, true),
    };
    if update {
        update_activity_on_interact(ess, host, u, broadcast);
    }
    Allow::Yes
}

// ===== Chat (EssentialsChat) ======================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChatType {
    Normal,
    Shout,
    Question,
}

impl ChatType {
    fn key(self) -> &'static str {
        match self {
            ChatType::Normal => "normal",
            ChatType::Shout => "shout",
            ChatType::Question => "question",
        }
    }
}

fn first_char(ess: &Ess, key: &str, default: char) -> char {
    ess.config_str(key).and_then(|s| s.chars().next()).unwrap_or(default)
}

/// User.isToggleShout.
fn toggle_shout(ess: &mut Ess, u: Uuid) -> bool {
    let default = ess.config_bool("chat.shout-default", false);
    if ess.config_bool("chat.persist-shout", false) {
        return ess.users.get(u).and_then(|x| x.get("shouting").and_then(Value::as_bool)).unwrap_or(default);
    }
    st(ess).shout.get(&u).copied().unwrap_or(default)
}

fn set_toggle_shout(ess: &mut Ess, u: Uuid, on: bool) {
    st(ess).shout.insert(u, on);
    if ess.config_bool("chat.persist-shout", false) {
        ess.users.user(u).set("shouting", json!(on));
    }
}

/// AbstractChatHandler.getChatType.
fn chat_type(ess: &mut Ess, u: Uuid, message: &str) -> ChatType {
    let Some(first) = message.chars().next() else { return ChatType::Normal };
    if message.chars().count() == 1 {
        return if toggle_shout(ess, u) { ChatType::Shout } else { ChatType::Normal };
    }
    if first == first_char(ess, "chat.question", '?') && ess.config_bool("chat.question-enabled", true) {
        ChatType::Question
    } else if first == first_char(ess, "chat.shout", '!') || toggle_shout(ess, u) {
        ChatType::Shout
    } else {
        ChatType::Normal
    }
}

/// Settings.getChatFormat(group, type): group-formats, then format, by chat type; `{0}`..`{9}` placeholders.
fn chat_format(ess: &Ess, group: &str, key: &str) -> String {
    let group_path = format!("chat.group-formats.{group}");
    let format = ess
        .config_str(&format!("{group_path}.{key}"))
        .or_else(|| ess.config_str(&group_path))
        .or_else(|| ess.config_str(&format!("chat.format.{key}")))
        .or_else(|| ess.config_str("chat.format"))
        .unwrap_or_else(|| "&7[{GROUP}]&r {DISPLAYNAME}&7:&r {MESSAGE}".to_string());
    let f = replace_format(&format)
        .replace("{DISPLAYNAME}", "%1$s")
        .replace("{MESSAGE}", "%2$s")
        .replace("{GROUP}", "{0}")
        .replace("{WORLDNAME}", "{1}")
        .replace("{WORLD}", "{1}")
        .replace("{SHORTWORLDNAME}", "{2}")
        .replace("{TEAMPREFIX}", "{3}")
        .replace("{TEAMSUFFIX}", "{4}")
        .replace("{TEAMNAME}", "{5}")
        .replace("{PREFIX}", "{6}")
        .replace("{SUFFIX}", "{7}")
        .replace("{USERNAME}", "{8}")
        .replace("{NICKNAME}", "{9}");
    format!("§r{f}")
}

/// Settings.getWorldAlias.
fn world_alias(ess: &Ess, world: &str) -> String {
    if let Some(Value::Object(map)) = ess.config("chat.world-aliases") {
        for (k, v) in map {
            if k.eq_ignore_ascii_case(world) {
                if let Some(s) = v.as_str() {
                    return replace_format(s);
                }
            }
        }
    }
    world.to_string()
}

/// java.lang.String.format for chat formats: `%1$s`, `%2$s`, `%s`, `%%`.
fn java_format(fmt: &str, args: &[&str]) -> String {
    let c: Vec<char> = fmt.chars().collect();
    let mut out = String::new();
    let mut next = 0;
    let mut i = 0;
    while i < c.len() {
        if c[i] == '%' {
            if c.get(i + 1) == Some(&'%') {
                out.push('%');
                i += 2;
                continue;
            }
            if c.get(i + 1) == Some(&'s') {
                out.push_str(args.get(next).copied().unwrap_or(""));
                next += 1;
                i += 2;
                continue;
            }
            if let (Some(d), Some('$'), Some('s')) = (c.get(i + 1).and_then(|d| d.to_digit(10)), c.get(i + 2), c.get(i + 3)) {
                out.push_str(args.get((d as usize).wrapping_sub(1)).copied().unwrap_or(""));
                i += 4;
                continue;
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// EssentialsPlayerListener.onPlayerChat (mute, ignore, activity) then EssentialsChat's format and recipients.
fn on_chat(ess: &mut Ess, host: &mut dyn Host, u: Uuid, raw: &str) -> Chat {
    let muted = is_muted(ess, u);
    if muted {
        notify_muted(ess, host, u);
        let name = name_of(ess, host, u);
        ess.send_tl(host, Sender::Console, "mutedUserSpeaks", &[Arg::Plain(name), Arg::Plain(raw.to_string())]);
    }
    let online = host.online();
    let mut recipients: Vec<Uuid> = Vec::new();
    for p in &online {
        if !ignores(ess, host, *p, u) {
            recipients.push(*p);
        }
    }
    if ess.config_bool("cancel-afk-on-chat", true) {
        ess.scheduler.later(0, Box::new(move |ess, host| update_activity(ess, host, u, true)));
    }
    set_display_nick(ess, host, u);
    if muted {
        return Chat::Cancel;
    }

    // handleChatFormat
    let who = Sender::Player(u);
    let kind = chat_type(ess, u, raw);
    let radius = ess.config_i64("chat.radius", ess.config_i64("chat-radius", 0)).max(0);
    let mut message = format_message(ess, host, who, "essentials.chat", raw);
    if strip_format(&message).is_empty() {
        return Chat::Cancel;
    }
    let group = ess.group(host, u);
    let loc = host.location(u).unwrap_or_else(|| Loc::new("minecraft:overworld", 0.0, 0.0, 0.0));
    let world = world_name(&loc.world);
    let username = name_of(ess, host, u);
    let nick = formatted_nickname(ess, u).unwrap_or_else(|| username.clone());
    let prefix = perm_affix(ess, host, u, false);
    let suffix = perm_affix(ess, host, u, true);
    let local = radius > 0 && kind == ChatType::Normal;
    let esc = |s: &str| s.replace('%', "%%");
    let format = chat_format(ess, &group, if local { "local" } else { kind.key() })
        .replace("{0}", &esc(&group))
        .replace("{1}", &esc(&world_alias(ess, &world)))
        .replace("{2}", &esc(&world.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()))
        .replace("{3}", "")
        .replace("{4}", "")
        .replace("{5}", "")
        .replace("{6}", &esc(&prefix))
        .replace("{7}", &esc(&suffix))
        .replace("{8}", &esc(&username))
        .replace("{9}", &esc(&nick));
    let base_mini = to_mini(&format);
    let mut format_mini = base_mini.clone();
    if radius > 0 {
        let first = message.chars().next();
        let strip_first = message.chars().count() > 1
            && ((kind == ChatType::Shout && first == Some(first_char(ess, "chat.shout", '!'))) || (kind == ChatType::Question && first == Some(first_char(ess, "chat.question", '?'))));
        if strip_first {
            message = message.chars().skip(1).collect::<String>().trim().to_string();
        }
        if strip_format(&message).is_empty() {
            return Chat::Cancel;
        }
        format_mini = if kind == ChatType::Normal {
            ess.tl("chatTypeLocal", &[]) + &format_mini
        } else {
            ess.tl(&format!("{}Format", kind.key()), &[Arg::Parsed(format_mini)])
        };
    }

    // handleChatRecipients
    let mut spies: Vec<Uuid> = Vec::new();
    if radius > 0 {
        if kind == ChatType::Normal {
            if !ess.has(host, who, "essentials.chat.local") {
                tell(ess, host, u, "notAllowedToLocal", &[]);
                return Chat::Cancel;
            }
            let mut keep = Vec::new();
            for p in recipients {
                if ess.has(host, Sender::Player(p), "essentials.chat.receive.local") {
                    keep.push(p);
                }
            }
            recipients = keep;
            let mut keep = Vec::new();
            for p in recipients {
                if p == u {
                    keep.push(p);
                    continue;
                }
                let near = host.location(p).is_some_and(|l| l.world == loc.world && (l.x - loc.x).powi(2) + (l.y - loc.y).powi(2) + (l.z - loc.z).powi(2) <= (radius * radius) as f64);
                if near {
                    keep.push(p);
                } else if ess.has(host, Sender::Player(p), "essentials.chat.spy") {
                    spies.push(p);
                }
            }
            recipients = keep;
            if recipients.len() < 2 {
                tell(ess, host, u, "localNoOne", &[]);
            }
        } else {
            let key = kind.key();
            if !ess.has(host, who, &format!("essentials.chat.{key}")) {
                let k = if kind == ChatType::Shout { "notAllowedToShout" } else { "notAllowedToQuestion" };
                tell(ess, host, u, k, &[]);
                return Chat::Cancel;
            }
            let mut keep = Vec::new();
            for p in recipients {
                if ess.has(host, Sender::Player(p), &format!("essentials.chat.receive.{key}")) {
                    keep.push(p);
                }
            }
            recipients = keep;
        }
    }

    let display_mini = to_mini(&display_name(ess, host, u));
    let message_mini = to_mini(&message);
    let line = ess.render(&java_format(&format_mini, &[&display_mini, &message_mini]));
    let everyone = recipients.len() == online.len();
    if radius == 0 && everyone && line.iter().all(|s| s.style == ess_text::Style::default()) && ess_text::plain(&line) == format!("<{username}> {raw}") {
        return Chat::Pass;
    }
    let mut lines = vec![(if everyone { None } else { Some(recipients) }, line)];
    if !spies.is_empty() {
        lines.push((Some(spies), ess.render(&java_format(&base_mini, &[&display_mini, &message_mini]))));
    }
    Chat::Replace(lines)
}

// ===== Private messages ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Response {
    Success,
    SuccessAfk,
    MessagesIgnored,
    SenderIgnored,
    Unreachable,
}

fn reachable(host: &mut dyn Host, who: Sender) -> bool {
    match who {
        Sender::Console => true,
        Sender::Player(u) => is_online(host, u),
    }
}

/// UserData.isLastMessageReplyRecipient.
fn reply_to_last(ess: &mut Ess, who: Sender) -> bool {
    let default = ess.config_bool("last-message-reply-recipient", false);
    match who {
        Sender::Player(u) => ess.users.get(u).and_then(|x| x.get("last-message-reply-recipient").and_then(Value::as_bool)).unwrap_or(default),
        Sender::Console => default,
    }
}

/// SimpleMessageRecipient.onReceiveMessage.
fn receive(ess: &mut Ess, host: &mut dyn Host, to: Sender, from: Sender, message: &str) -> Response {
    if !reachable(host, to) {
        return Response::Unreachable;
    }
    let mut afk = false;
    if let Sender::Player(t) = to {
        if st(ess).ignore_msg.contains(&t) && from.player().is_some() && !ess.has(host, from, "essentials.msgtoggle.bypass") {
            return Response::MessagesIgnored;
        }
        afk = is_afk(ess, t);
        if let Sender::Player(f) = from {
            if ignores(ess, host, t, f) {
                return Response::SenderIgnored;
            }
        }
    }
    let from_display = display_of(ess, host, from);
    let me = ess.tl("meRecipient", &[]);
    ess.send_tl(host, to, "msgFormat", &[Arg::Plain(from_display), Arg::Parsed(me), Arg::Plain(message.to_string())]);
    let now = host.now_millis();
    if reply_to_last(ess, to) {
        let timeout = ess.config_i64("last-message-reply-recipient-timeout", 180) * 1000;
        let current = st(ess).reply.get(&to).copied();
        let last = st(ess).last_message.get(&to).copied().unwrap_or(0);
        if current.is_none_or(|c| !reachable(host, c)) || now - last > timeout {
            st(ess).reply.insert(to, from);
        }
    } else {
        st(ess).reply.insert(to, from);
    }
    st(ess).last_message.insert(to, now);
    if afk { Response::SuccessAfk } else { Response::Success }
}

/// SimpleMessageRecipient.sendSocialSpy.
fn social_spy(ess: &mut Ess, host: &mut dyn Host, from: Sender, to: Sender, message: &str, muted: bool) {
    if !ess.config_bool("socialspy-messages", true) || (muted && !ess.config_bool("socialspy-listen-muted-players", true)) {
        return;
    }
    let (Sender::Player(f), Sender::Player(t)) = (from, to) else { return };
    if ess.has(host, from, "essentials.chat.spy.exempt") || ess.has(host, to, "essentials.chat.spy.exempt") {
        return;
    }
    let names = ess.config_bool("socialspy-uses-displaynames", true);
    let sn = if names { display_name(ess, host, f) } else { name_of(ess, host, f) };
    let rn = if names { display_name(ess, host, t) } else { name_of(ess, host, t) };
    let prefix = ess.tl(if muted { "socialSpyMutedPrefix" } else { "socialSpyPrefix" }, &[]);
    let body = ess.tl("socialSpyMsgFormat", &[Arg::Plain(sn), Arg::Plain(rn), Arg::Plain(message.to_string())]);
    for s in host.online() {
        if s != f && s != t && user_bool(ess, s, "socialspy") {
            ess.send_mini(host, Sender::Player(s), &format!("{prefix}{body}"));
        }
    }
}

/// SimpleMessageRecipient.sendMessage.
fn private_message(ess: &mut Ess, host: &mut dyn Host, from: Sender, to: Sender, message: &str) {
    if let Sender::Player(f) = from {
        if is_muted(ess, f) {
            social_spy(ess, host, from, to, message, true);
            notify_muted(ess, host, f);
            return;
        }
    }
    let response = receive(ess, host, to, from, message);
    match response {
        Response::Unreachable => {
            let d = display_of(ess, host, to);
            ess.send_tl(host, from, "recentlyForeverAlone", &[Arg::Plain(d)]);
        }
        Response::MessagesIgnored => {
            let d = display_of(ess, host, to);
            ess.send_tl(host, from, "msgIgnore", &[Arg::Plain(d)]);
        }
        Response::SenderIgnored => {}
        Response::Success | Response::SuccessAfk => {
            let d = display_of(ess, host, to);
            if response == Response::SuccessAfk {
                let afk_msg = to.player().and_then(|t| st(ess).afk_message.get(&t).cloned());
                match afk_msg {
                    Some(m) => ess.send_tl(host, from, "userAFKWithMessage", &[Arg::Plain(d.clone()), Arg::Plain(m)]),
                    None => ess.send_tl(host, from, "userAFK", &[Arg::Plain(d.clone())]),
                }
            }
            let me = ess.tl("meSender", &[]);
            ess.send_tl(host, from, "msgFormat", &[Arg::Parsed(me), Arg::Plain(d), Arg::Plain(message.to_string())]);
            social_spy(ess, host, from, to, message, false);
            st(ess).reply.insert(from, to);
        }
    }
}

fn msg_text(ctx: &mut Ctx, text: &str) -> String {
    match ctx.sender {
        Sender::Player(_) => format_message(ctx.ess, ctx.host, ctx.sender, "essentials.msg", text),
        Sender::Console => replace_format(text),
    }
}

fn cmd_msg(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let wildcard = ctx.has("essentials.msg.multiple");
    let message = msg_text(ctx, &final_arg(args, 1));
    let console_name = ctx.ess.tl("consoleName", &[]);
    if args[0].eq_ignore_ascii_case("Console") || args[0].eq_ignore_ascii_case(&console_name) {
        private_message(ctx.ess, ctx.host, ctx.sender, Sender::Console, &message);
        return Ok(());
    }
    for t in loop_online(ctx, &args[0], wildcard)? {
        private_message(ctx.ess, ctx.host, ctx.sender, Sender::Player(t), &message);
    }
    Ok(())
}

/// EssentialsLoopCommand.loopOnlinePlayers (single matches).
fn loop_online(ctx: &mut Ctx, search: &str, wildcard: bool) -> Result<Vec<Uuid>, CmdError> {
    if search.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    if let (Sender::Player(u), "@s" | "@p") = (ctx.sender, search) {
        return Ok(vec![u]);
    }
    if wildcard && (search == "*" || search == "**") {
        let mut out = Vec::new();
        for p in ctx.host.online() {
            if !hidden_from(ctx.ess, ctx.host, ctx.sender, p) {
                out.push(p);
            }
        }
        return Ok(out);
    }
    Ok(vec![ctx.online_player(search)?])
}

fn cmd_r(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let message = msg_text(ctx, &final_arg(args, 0));
    let Some(target) = st(ctx.ess).reply.get(&ctx.sender).copied() else {
        return Err(CmdError::Tl("foreverAlone", vec![]));
    };
    private_message(ctx.ess, ctx.host, ctx.sender, target, &message);
    Ok(())
}

// ===== Toggle commands (EssentialsToggleCommand) ==================================================

type ToggleFn = fn(&mut Ess, &mut dyn Host, Sender, Uuid, Option<bool>);

fn match_toggle(arg: &str) -> Option<bool> {
    if arg.eq_ignore_ascii_case("on") || arg.starts_with("ena") || arg == "1" {
        Some(true)
    } else if arg.eq_ignore_ascii_case("off") || arg.starts_with("dis") || arg == "0" {
        Some(false)
    } else {
        None
    }
}

fn toggle_command(ctx: &mut Ctx, args: &[String], others: &str, f: ToggleFn) -> CmdResult {
    let Sender::Player(u) = ctx.sender else { return toggle_others(ctx, args, f) };
    if args.len() == 1 {
        let t = match_toggle(&args[0]);
        if t.is_none() && ctx.has(others) {
            return toggle_others(ctx, args, f);
        }
        f(ctx.ess, ctx.host, ctx.sender, u, t);
    } else if args.len() == 2 && ctx.has(others) {
        return toggle_others(ctx, args, f);
    } else {
        f(ctx.ess, ctx.host, ctx.sender, u, None);
    }
    Ok(())
}

fn toggle_others(ctx: &mut Ctx, args: &[String], f: ToggleFn) -> CmdResult {
    if args.is_empty() || args[0].trim().chars().count() < 2 {
        return Err(CmdError::PlayerNotFound);
    }
    let mut found = false;
    for p in match_players(ctx.host, &args[0]) {
        if hidden_from(ctx.ess, ctx.host, ctx.sender, p) {
            continue;
        }
        found = true;
        let t = args.get(1).and_then(|a| match_toggle(a));
        f(ctx.ess, ctx.host, ctx.sender, p, t);
    }
    if found { Ok(()) } else { Err(CmdError::PlayerNotFound) }
}

fn for_other(sender: Sender, u: Uuid) -> bool {
    sender != Sender::Player(u)
}

fn toggle_msg(ess: &mut Ess, host: &mut dyn Host, sender: Sender, u: Uuid, enabled: Option<bool>) {
    let enabled = enabled.unwrap_or(!st(ess).ignore_msg.contains(&u));
    if enabled {
        st(ess).ignore_msg.insert(u);
    } else {
        st(ess).ignore_msg.remove(&u);
    }
    tell(ess, host, u, if !enabled { "msgEnabled" } else { "msgDisabled" }, &[]);
    if for_other(sender, u) {
        let d = display_name(ess, host, u);
        ess.send_tl(host, sender, if !enabled { "msgEnabledFor" } else { "msgDisabledFor" }, &[Arg::Plain(d)]);
    }
}

fn cmd_msgtoggle(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.msgtoggle.others", toggle_msg)
}

fn toggle_reply(ess: &mut Ess, host: &mut dyn Host, sender: Sender, u: Uuid, enabled: Option<bool>) {
    let enabled = enabled.unwrap_or(!reply_to_last(ess, Sender::Player(u)));
    ess.users.user(u).set("last-message-reply-recipient", json!(enabled));
    tell(ess, host, u, if !enabled { "replyLastRecipientDisabled" } else { "replyLastRecipientEnabled" }, &[]);
    if for_other(sender, u) {
        let d = display_name(ess, host, u);
        ess.send_tl(host, sender, if !enabled { "replyLastRecipientDisabledFor" } else { "replyLastRecipientEnabledFor" }, &[Arg::Plain(d)]);
    }
}

fn cmd_rtoggle(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.rtoggle.others", toggle_reply)
}

fn toggle_spy(ess: &mut Ess, host: &mut dyn Host, sender: Sender, u: Uuid, enabled: Option<bool>) {
    let enabled = enabled.unwrap_or(!user_bool(ess, u, "socialspy"));
    ess.users.user(u).set("socialspy", json!(enabled));
    let d = display_name(ess, host, u);
    let state = ess.tl(if enabled { "enabled" } else { "disabled" }, &[]);
    let args = [Arg::Plain(d), Arg::Parsed(state)];
    tell(ess, host, u, "socialSpy", &args);
    if for_other(sender, u) {
        ess.send_tl(host, sender, "socialSpy", &args);
    }
}

fn cmd_socialspy(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.socialspy.others", toggle_spy)
}

fn toggle_shout_cmd(ess: &mut Ess, host: &mut dyn Host, sender: Sender, u: Uuid, enabled: Option<bool>) {
    let enabled = enabled.unwrap_or(!toggle_shout(ess, u));
    set_toggle_shout(ess, u, enabled);
    tell(ess, host, u, if enabled { "shoutEnabled" } else { "shoutDisabled" }, &[]);
    if for_other(sender, u) {
        let d = display_name(ess, host, u);
        ess.send_tl(host, sender, if enabled { "shoutEnabledFor" } else { "shoutDisabledFor" }, &[Arg::Plain(d)]);
    }
}

fn cmd_toggleshout(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    toggle_command(ctx, args, "essentials.toggleshout.others", toggle_shout_cmd)
}

// ===== afk, ignore, realname, nick ================================================================

fn cmd_afk(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    match ctx.sender {
        Sender::Player(u) => {
            if !args.is_empty() && ctx.has("essentials.afk.others") {
                let (target, message) = match ctx.online_player(&args[0]) {
                    Ok(t) => (t, (args.len() > 1).then(|| final_arg(args, 1))),
                    Err(_) => (u, Some(final_arg(args, 0))),
                };
                toggle_afk(ctx.ess, ctx.host, Some(u), target, message)
            } else {
                let message = (!args.is_empty()).then(|| final_arg(args, 0));
                toggle_afk(ctx.ess, ctx.host, Some(u), u, message)
            }
        }
        Sender::Console => {
            if args.is_empty() {
                return Err(CmdError::Usage(None));
            }
            let target = ctx.online_player(&args[0])?;
            let message = (args.len() > 1).then(|| final_arg(args, 1));
            toggle_afk(ctx.ess, ctx.host, None, target, message)
        }
    }
}

fn toggle_afk(ess: &mut Ess, host: &mut dyn Host, sender: Option<Uuid>, u: Uuid, message: Option<String>) -> CmdResult {
    if let (Some(_), Some(s)) = (&message, sender) {
        if is_muted(ess, s) {
            return Err(muted_error(ess, host, s));
        }
        if !ess.has(host, Sender::Player(s), "essentials.afk.message") {
            return Err(CmdError::Tl("noPermToAFKMessage", vec![]));
        }
    }
    set_display_nick(ess, host, u);
    let before = is_afk(ess, u);
    set_afk(ess, host, u, !before);
    let after = is_afk(ess, u);
    if before == after {
        return Ok(());
    }
    let hidden = ess.is_vanished(u);
    let keys = if !after {
        update_activity(ess, host, u, false);
        (!hidden).then_some(("userIsNotAway", "userIsNotAwaySelf"))
    } else {
        if let Some(m) = &message {
            st(ess).afk_message.insert(u, m.clone());
        }
        (!hidden).then_some(if message.is_some() { ("userIsAwayWithMessage", "userIsAwaySelfWithMessage") } else { ("userIsAway", "userIsAwaySelf") })
    };
    if let Some((key, self_key)) = keys {
        let display = display_name(ess, host, u);
        let args = [Arg::Plain(display), Arg::Plain(message.unwrap_or_default())];
        if ess.config_bool("broadcast-afk-message", true) {
            broadcast_tl(ess, host, Some(u), None, Some(u), key, &args);
        }
        tell(ess, host, u, self_key, &args);
    }
    set_display_nick(ess, host, u);
    Ok(())
}

fn cmd_ignore(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let u = ctx.player()?;
    if args.is_empty() {
        let mut names = Vec::new();
        for id in ignore_list(ctx.ess, u) {
            if let Some(p) = parse_uuid(&id) {
                let n = ctx.ess.users.get(p).and_then(|x| x.name()).or_else(|| ctx.host.name(p));
                if let Some(n) = n.filter(|n| !n.trim().is_empty()) {
                    names.push(n);
                }
            }
        }
        let list = names.join(" ");
        ctx.tl(if list.is_empty() { "noIgnored" } else { "ignoredList" }, &[Arg::Plain(list)]);
        return Ok(());
    }
    let p = ctx.any_player(&args[0])?;
    let pname = name_of(ctx.ess, ctx.host, p);
    let mut list = ignore_list(ctx.ess, u);
    let id = uuid_string(p);
    if ctx.ess.has(ctx.host, Sender::Player(p), "essentials.chat.ignoreexempt") {
        ctx.tl("ignoreExempt", &[]);
    } else if list.contains(&id) {
        list.retain(|x| *x != id);
        ctx.ess.users.user(u).set("ignore", json!(list));
        ctx.tl("unignorePlayer", &[Arg::Plain(pname)]);
    } else if p == u {
        ctx.tl("ignoreYourself", &[]);
    } else {
        list.push(id);
        ctx.ess.users.user(u).set("ignore", json!(list));
        ctx.tl("ignorePlayer", &[Arg::Plain(pname)]);
    }
    Ok(())
}

fn cmd_realname(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let lookup = args[0].to_lowercase();
    let mut found = false;
    for u in ctx.host.online() {
        if hidden_from(ctx.ess, ctx.host, ctx.sender, u) {
            continue;
        }
        set_display_nick(ctx.ess, ctx.host, u);
        let d = display_name(ctx.ess, ctx.host, u);
        if strip_format(&d).to_lowercase().contains(&lookup) {
            found = true;
            let n = name_of(ctx.ess, ctx.host, u);
            ctx.tl("realName", &[Arg::Plain(d), Arg::Plain(n)]);
        }
    }
    if found { Ok(()) } else { Err(CmdError::PlayerNotFound) }
}

fn cmd_nick(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    match ctx.sender {
        Sender::Player(u) => {
            if args.is_empty() {
                return Err(CmdError::Usage(None));
            }
            if args.len() > 1 && ctx.has("essentials.nick.others") {
                let nick = format_nickname(ctx.ess, ctx.host, Some(u), &args[1])?;
                for t in loop_offline(ctx, &args[0])? {
                    update_nick(ctx.ess, ctx.host, t, &nick)?;
                }
                ctx.tl("nickChanged", &[]);
            } else {
                let nick = format_nickname(ctx.ess, ctx.host, Some(u), &args[0])?;
                update_nick(ctx.ess, ctx.host, u, &nick)?;
            }
        }
        Sender::Console => {
            if args.len() < 2 {
                return Err(CmdError::Usage(None));
            }
            let nick = format_nickname(ctx.ess, ctx.host, None, &args[1])?;
            for t in loop_offline(ctx, &args[0])? {
                update_nick(ctx.ess, ctx.host, t, &nick)?;
            }
            ctx.tl("nickChanged", &[]);
        }
    }
    Ok(())
}

/// EssentialsLoopCommand.loopOfflinePlayers (single matches, wildcards).
fn loop_offline(ctx: &mut Ctx, search: &str) -> Result<Vec<Uuid>, CmdError> {
    if search.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    if let (Sender::Player(u), "@s" | "@p") = (ctx.sender, search) {
        return Ok(vec![u]);
    }
    if let Some(u) = parse_uuid(search) {
        return if ctx.ess.users.exists(u) { Ok(vec![u]) } else { Err(CmdError::PlayerNotFound) };
    }
    if search == "**" {
        return Ok(ctx.ess.users.all());
    }
    if search == "*" {
        let mut out = Vec::new();
        for p in ctx.host.online() {
            if !hidden_from(ctx.ess, ctx.host, ctx.sender, p) {
                out.push(p);
            }
        }
        return Ok(out);
    }
    Ok(vec![ctx.any_player(search)?])
}

fn nick_regex(ess: &Ess) -> Regex {
    Regex::new(&cfg_str(ess, "allowed-nicks-regex", "^[a-zA-Z_0-9§]+$")).unwrap_or_else(|| Regex::new("^[a-zA-Z_0-9§]+$").expect("default nick regex"))
}

/// Commandnick.formatNickname.
fn format_nickname(ess: &mut Ess, host: &mut dyn Host, user: Option<Uuid>, nick: &str) -> Result<String, CmdError> {
    let new = match user {
        None => replace_format(nick),
        Some(u) => format_string(ess, host, Sender::Player(u), "essentials.nick", nick),
    };
    let who = user.map(Sender::Player);
    let has = |ess: &mut Ess, host: &mut dyn Host, node: &str| who.is_some_and(|w| ess.has(host, w, node));
    let length = if ess.config_bool("ignore-colors-in-max-nick-length", false) { strip_format(&new).chars().count() } else { unformat_string(&new).chars().count() };
    if !nick_regex(ess).is_match(&new) && user.is_some() && !has(ess, host, "essentials.nick.allowunsafe") {
        return Err(CmdError::Tl("nickNamesAlpha", vec![]));
    }
    if length as i64 > ess.config_i64("max-nick-length", 30) {
        return Err(CmdError::Tl("nickTooLong", vec![]));
    }
    if strip_format(&new).is_empty() {
        return Err(CmdError::Tl("nickNamesAlpha", vec![]));
    }
    if let Some(u) = user {
        let name = name_of(ess, host, u);
        if has(ess, host, "essentials.nick.changecolors") && !has(ess, host, "essentials.nick.changecolors.bypass") && strip_format(&new) != name && !nick.eq_ignore_ascii_case("off") {
            return Err(CmdError::Tl("nickNamesOnlyColorChanges", vec![]));
        }
        if !has(ess, host, "essentials.nick.blacklist.bypass") && ess.config_list("nick-blacklist").iter().any(|b| Regex::new(b).is_some_and(|r| r.find_in(&new))) {
            return Err(CmdError::Tl("nickNameBlacklist", vec![Arg::Plain(nick.to_string())]));
        }
    }
    Ok(new)
}

/// Commandnick.nickInUse.
fn nick_in_use(ess: &mut Ess, host: &mut dyn Host, target: Uuid, nick: &str) -> bool {
    let lower = strip_format(&nick.to_lowercase());
    let tname = name_of(ess, host, target);
    for p in host.online() {
        let pname = name_of(ess, host, p);
        if pname == tname {
            continue;
        }
        let other = nickname(ess, p).map(|n| strip_format(&n).to_lowercase());
        if other.is_some_and(|n| !n.is_empty() && n == lower) || lower == pname.to_lowercase() {
            return true;
        }
    }
    ess.users.names.get(&lower).is_some_and(|u| *u != target)
}

/// Commandnick.updatePlayer.
fn update_nick(ess: &mut Ess, host: &mut dyn Host, target: Uuid, nick: &str) -> CmdResult {
    let name = name_of(ess, host, target);
    let shown = |ess: &mut Ess, host: &mut dyn Host| if ess.config_bool("change-displayname", true) { display_name(ess, host, target) } else { nick.to_string() };
    if nick.eq_ignore_ascii_case("off") {
        ess.users.user(target).set("nickname", Value::Null);
        set_display_nick(ess, host, target);
        tell(ess, host, target, "nickNoMore", &[]);
    } else if name.eq_ignore_ascii_case(nick) {
        ess.users.user(target).set("nickname", json!(nick));
        set_display_nick(ess, host, target);
        let d = display_name(ess, host, target);
        if !strip_format(&d).eq_ignore_ascii_case(&name) {
            tell(ess, host, target, "nickNoMore", &[]);
        }
        let s = shown(ess, host);
        tell(ess, host, target, "nickSet", &[Arg::Plain(s)]);
    } else if nick_in_use(ess, host, target, nick) {
        return Err(CmdError::Usage(Some(ess.tl("nickInUse", &[]))));
    } else {
        ess.users.user(target).set("nickname", json!(nick));
        set_display_nick(ess, host, target);
        let s = shown(ess, host);
        tell(ess, host, target, "nickSet", &[Arg::Plain(s)]);
    }
    Ok(())
}

// ===== me, helpop, broadcast, broadcastworld ======================================================

fn cmd_me(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(u) = ctx.sender else {
        if args.is_empty() {
            return Err(CmdError::Usage(None));
        }
        let message = replace_format(&final_arg(args, 0));
        broadcast_tl(ctx.ess, ctx.host, None, None, None, "action", &[Arg::Plain("@".into()), Arg::Plain(message)]);
        return Ok(());
    };
    if is_muted(ctx.ess, u) {
        return Err(muted_error(ctx.ess, ctx.host, u));
    }
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let message = format_message(ctx.ess, ctx.host, ctx.sender, "essentials.chat", &final_arg(args, 0));
    set_display_nick(ctx.ess, ctx.host, u);
    let display = display_name(ctx.ess, ctx.host, u);
    let tl_args = [Arg::Plain(display), Arg::Plain(message)];
    let radius = ctx.ess.config_i64("chat.radius", ctx.ess.config_i64("chat-radius", 0));
    if radius < 1 {
        broadcast_tl(ctx.ess, ctx.host, None, None, None, "action", &tl_args);
        return Ok(());
    }
    let at = ctx.host.location(u);
    let mut out = Vec::new();
    for p in ctx.host.online() {
        if p == u {
            out.push(p);
            continue;
        }
        let near = match (ctx.host.location(p), &at) {
            (Some(l), Some(a)) => l.world == a.world && (l.x - a.x).powi(2) + (l.y - a.y).powi(2) + (l.z - a.z).powi(2) <= (radius * radius) as f64,
            _ => false,
        };
        if !near || ignores(ctx.ess, ctx.host, p, u) {
            if ctx.ess.has(ctx.host, Sender::Player(p), "essentials.chat.spy") {
                out.push(p);
            }
        } else {
            out.push(p);
        }
    }
    if out.len() < 2 {
        ctx.tl("localNoOne", &[]);
    }
    for p in out {
        ctx.ess.send_tl(ctx.host, Sender::Player(p), "action", &tl_args);
    }
    Ok(())
}

fn cmd_helpop(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if let Sender::Player(u) = ctx.sender {
        set_display_nick(ctx.ess, ctx.host, u);
    }
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let message = strip_format(&final_arg(args, 0));
    let from = display_of(ctx.ess, ctx.host, ctx.sender);
    let tl_args = [Arg::Plain(from), Arg::Plain(message)];
    ctx.ess.send_tl(ctx.host, Sender::Console, "helpOp", &tl_args);
    let mut recipients = Vec::new();
    for p in ctx.host.online() {
        if ctx.ess.has(ctx.host, Sender::Player(p), "essentials.helpop.receive") {
            recipients.push(p);
        }
    }
    if let Sender::Player(u) = ctx.sender {
        if !recipients.contains(&u) {
            ctx.tl("helpOp", &tl_args);
        }
    }
    for p in recipients {
        ctx.ess.send_tl(ctx.host, Sender::Player(p), "helpOp", &tl_args);
    }
    Ok(())
}

fn cmd_broadcast(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let message = replace_format(&final_arg(args, 0)).replace("\\n", "\n");
    let name = display_of(ctx.ess, ctx.host, ctx.sender);
    broadcast_tl(ctx.ess, ctx.host, None, None, None, "broadcast", &[Arg::Parsed(to_mini(&message)), Arg::Plain(name)]);
    Ok(())
}

fn cmd_broadcastworld(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let (world, name, message) = match ctx.sender {
        Sender::Player(u) => {
            let mut world = ctx.host.location(u).map(|l| l.world).unwrap_or_default();
            let mut message = final_arg(args, 0);
            if ctx.ess.config_bool("allow-world-in-broadcastworld", true) {
                if let Some(w) = find_world(ctx.host, &args[0]) {
                    world = w;
                    message = final_arg(args, 1);
                }
            }
            (world, display_name(ctx.ess, ctx.host, u), message)
        }
        Sender::Console => {
            let Some(w) = find_world(ctx.host, &args[0]) else { return Err(CmdError::Tl("invalidWorld", vec![])) };
            (w, "CONSOLE".to_string(), final_arg(args, 1))
        }
    };
    if message.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let formatted = replace_format(&message).replace("\\n", "\n");
    let tl_args = [Arg::Parsed(to_mini(&formatted)), Arg::Parsed(to_mini(&name))];
    for p in ctx.host.online() {
        if ctx.host.location(p).is_some_and(|l| l.world == world) {
            ctx.ess.send_tl(ctx.host, Sender::Player(p), "broadcast", &tl_args);
        }
    }
    Ok(())
}

// ===== mute =======================================================================================

/// The uuid Bukkit gives a name that never joined (an offline player stub).
fn stub_uuid(name: &str) -> Uuid {
    let mut h: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for b in format!("OfflinePlayer:{name}").bytes() {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    (h & !(0xf << 76) | (0x3 << 76)) & !(0x3 << 62) | (0x2 << 62)
}

fn cmd_mute(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let unmute = ctx.label.contains("unmute");
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let (user, nomatch) = match ctx.any_player(&args[0]) {
        Ok(u) => (u, false),
        Err(_) => {
            let u = stub_uuid(&args[0]);
            ctx.ess.users.user(u).set("last-account-name", json!(args[0]));
            (u, true)
        }
    };
    let display = display_name(ctx.ess, ctx.host, user);
    let uname = name_of(ctx.ess, ctx.host, user);
    if unmute && !is_muted(ctx.ess, user) {
        ctx.tl("playerNotMuted", &[Arg::Plain(display)]);
        return Ok(());
    }
    if !is_online(ctx.host, user) && ctx.is_player() {
        if !ctx.has("essentials.mute.offline") {
            return Err(CmdError::Tl("muteExemptOffline", vec![]));
        }
    } else if !unmute && ctx.ess.has(ctx.host, Sender::Player(user), "essentials.mute.exempt") {
        return Err(CmdError::Tl("muteExempt", vec![]));
    }
    let now = ctx.host.now_millis();
    let mut timestamp = 0;
    let mut reason = None;
    if !unmute && args.len() > 1 {
        match parse_date_diff(now, &args[1], true) {
            Some(t) => {
                timestamp = t;
                reason = Some(final_arg(args, 2));
            }
            None => reason = Some(final_arg(args, 1)),
        }
        let max = ctx.ess.config_i64("max-mute-time", -1) * 1000;
        if max > 0 && timestamp - now > max && ctx.is_player() && !ctx.has("essentials.mute.unlimited") {
            ctx.tl("oversizedMute", &[]);
            return Err(CmdError::Quiet);
        }
    }
    {
        let muted_now = is_muted(ctx.ess, user);
        let data = ctx.ess.users.user(user);
        match &reason {
            Some(r) => {
                data.set("mute-reason", if r.is_empty() { Value::Null } else { json!(r) });
                data.set("muted", json!(true));
            }
            None => {
                data.set("muted", json!(!muted_now));
                if muted_now {
                    data.set("mute-reason", Value::Null);
                }
            }
        }
        data.set("timestamps.mute", json!(timestamp));
    }
    let muted = is_muted(ctx.ess, user);
    let time = format_date_diff(now, timestamp);
    if nomatch {
        ctx.tl("userUnknown", &[Arg::Plain(uname.clone())]);
    }
    let sender_name = match ctx.sender {
        Sender::Player(p) => name_of(ctx.ess, ctx.host, p),
        Sender::Console => "CONSOLE".to_string(),
    };
    if muted {
        let reason = mute_reason(ctx.ess, user);
        let (d, t) = (Arg::Plain(display.clone()), Arg::Plain(time.clone()));
        match (&reason, timestamp > 0) {
            (None, true) => {
                ctx.tl("mutedPlayerFor", &[d, t.clone()]);
                tell(ctx.ess, ctx.host, user, "playerMutedFor", &[t]);
            }
            (Some(r), true) => {
                ctx.tl("mutedPlayerForReason", &[d, t.clone(), Arg::Plain(r.clone())]);
                tell(ctx.ess, ctx.host, user, "playerMutedForReason", &[t, Arg::Plain(r.clone())]);
            }
            (None, false) => {
                ctx.tl("mutedPlayer", &[d]);
                tell(ctx.ess, ctx.host, user, "playerMuted", &[]);
            }
            (Some(r), false) => {
                ctx.tl("mutedPlayerReason", &[d, Arg::Plain(r.clone())]);
                tell(ctx.ess, ctx.host, user, "playerMutedReason", &[Arg::Plain(r.clone())]);
            }
        }
        let (key, notify): (&str, Vec<Arg>) = match (&reason, timestamp > 0) {
            (Some(r), true) => ("muteNotifyForReason", vec![Arg::Plain(sender_name), Arg::Plain(uname), Arg::Plain(time), Arg::Plain(r.clone())]),
            (Some(r), false) => ("muteNotify", vec![Arg::Plain(sender_name), Arg::Plain(uname), Arg::Plain(r.clone())]),
            (None, true) => ("muteNotifyFor", vec![Arg::Plain(sender_name), Arg::Plain(uname), Arg::Plain(time)]),
            (None, false) => ("muteNotify", vec![Arg::Plain(sender_name), Arg::Plain(uname), Arg::Plain(time)]),
        };
        ctx.ess.send_tl(ctx.host, Sender::Console, key, &notify);
        broadcast_tl(ctx.ess, ctx.host, None, Some("essentials.mute.notify"), None, key, &notify);
    } else {
        ctx.tl("unmutedPlayer", &[Arg::Plain(display)]);
        tell(ctx.ess, ctx.host, user, "playerUnmuted", &[]);
        let notify = [Arg::Plain(sender_name), Arg::Plain(uname)];
        ctx.ess.send_tl(ctx.host, Sender::Console, "unmuteNotify", &notify);
        broadcast_tl(ctx.ess, ctx.host, None, Some("essentials.mute.notify"), None, "unmuteNotify", &notify);
    }
    Ok(())
}

// ===== Mail =======================================================================================

fn mails(ess: &mut Ess, u: Uuid) -> Vec<Value> {
    match ess.users.get(u).and_then(|x| x.get("mail").cloned()) {
        Some(Value::Array(a)) => a,
        _ => Vec::new(),
    }
}

fn set_mails(ess: &mut Ess, u: Uuid, list: Vec<Value>) {
    ess.users.user(u).set("mail", if list.is_empty() { Value::Null } else { Value::Array(list) });
}

fn mail_field(m: &Value, key: &str) -> String {
    match m.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}

fn mail_expired(m: &Value, now: i64) -> bool {
    let expire = m.get("expire").and_then(Value::as_i64).unwrap_or(0);
    expire != 0 && expire < now
}

fn unread_mail(ess: &mut Ess, host: &mut dyn Host, u: Uuid) -> usize {
    let now = host.now_millis();
    mails(ess, u).iter().filter(|m| !m.get("read").and_then(Value::as_bool).unwrap_or(false) && !mail_expired(m, now)).count()
}

/// MailServiceImpl.sendMail: the newest first.
fn send_mail(ess: &mut Ess, host: &mut dyn Host, to: Uuid, from: (String, Option<Uuid>), message: &str, expire: i64) {
    let now = host.now_millis();
    let mail = json!({
        "legacy": false,
        "read": false,
        "message": message,
        "sender-name": from.0,
        "sender-uuid": from.1.map(uuid_string).unwrap_or_else(|| "null".into()),
        "timestamp": now,
        "expire": expire,
    });
    let mut list = mails(ess, to);
    list.insert(0, mail);
    set_mails(ess, to, list);
}

/// User.notifyOfMail: called by the core before each command of a player with essentials.mail.
pub fn notify_mail(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    let unread = unread_mail(ess, host, player);
    if unread == 0 {
        return;
    }
    let cooldown = ess.config_i64("notify-player-of-mail-cooldown", 0) * 1000;
    let now = host.now_millis();
    let last = st(ess).mail_notified.get(&player).copied().unwrap_or(0);
    if now - last >= cooldown {
        tell(ess, host, player, "youHaveNewMail", &[Arg::from(unread)]);
        st(ess).mail_notified.insert(player, now);
    }
}

fn mail_date(ms: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(ms).single().map(|d| d.format("%Y/%m/%d %H:%M").to_string()).unwrap_or_default()
}

/// Commandmail's mailsPerMinute check.
fn mail_rate(ess: &mut Ess, host: &mut dyn Host) -> CmdResult {
    let now = host.now_millis();
    let max = ess.config_i64("mails-per-minute", 1000);
    let s = st(ess);
    if (now - s.mail_minute.0).abs() > 60_000 {
        s.mail_minute = (now, 0);
    }
    s.mail_minute.1 += 1;
    if s.mail_minute.1 > max {
        return Err(CmdError::Tl("mailDelay", vec![Arg::from(max)]));
    }
    Ok(())
}

fn never_on_server(name: &str) -> CmdError {
    CmdError::Tl("playerNeverOnServer", vec![Arg::Plain(name.to_string())])
}

fn mail_send_all(ess: &mut Ess, host: &mut dyn Host, from: (String, Option<Uuid>), message: &str, expire: i64) {
    for u in ess.users.all() {
        send_mail(ess, host, u, from.clone(), message, expire);
    }
}

fn clear_mail(ctx: &mut Ctx, target: Uuid, index: Option<usize>) -> CmdResult {
    let list = mails(ctx.ess, target);
    if list.is_empty() {
        let d = display_name(ctx.ess, ctx.host, target);
        let own = ctx.sender == Sender::Player(target);
        ctx.tl(if own { "noMail" } else { "noMailOther" }, &[Arg::Plain(d)]);
        return Err(CmdError::Quiet);
    }
    match index {
        Some(i) => {
            if i > list.len() {
                ctx.tl("mailClearIndex", &[Arg::from(list.len())]);
                return Err(CmdError::Quiet);
            }
            let mut list = list;
            list.remove(i - 1);
            set_mails(ctx.ess, target, list);
        }
        None => set_mails(ctx.ess, target, Vec::new()),
    }
    ctx.tl("mailCleared", &[]);
    Ok(())
}

fn positive_int(s: &str) -> Option<usize> {
    s.parse::<usize>().ok().filter(|n| *n > 0)
}

fn cmd_mail(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let sub = args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let now = ctx.host.now_millis();
    let Sender::Player(u) = ctx.sender else { return mail_console(ctx, args, &sub) };
    let me = (name_of(ctx.ess, ctx.host, u), Some(u));
    if sub == "read" {
        let list = mails(ctx.ess, u);
        if list.is_empty() {
            ctx.tl("noMail", &[]);
            return Err(CmdError::Quiet);
        }
        let mut kept = Vec::new();
        let mut lines = Vec::new();
        for mut m in list {
            if mail_expired(&m, now) {
                continue;
            }
            let legacy = m.get("legacy").and_then(Value::as_bool).unwrap_or(false);
            let message = mail_field(&m, "message");
            if legacy {
                lines.push(ctx.ess.tl("mailMessage", &[Arg::Plain(message)]));
            } else {
                let read = m.get("read").and_then(Value::as_bool).unwrap_or(false);
                let timed = m.get("expire").and_then(Value::as_i64).unwrap_or(0) != 0;
                let key = match (read, timed) {
                    (false, false) => "mailFormatNew",
                    (false, true) => "mailFormatNewTimed",
                    (true, false) => "mailFormatNewRead",
                    (true, true) => "mailFormatNewReadTimed",
                };
                let date = mail_date(m.get("timestamp").and_then(Value::as_i64).unwrap_or(0));
                lines.push(ctx.ess.tl(key, &[Arg::Plain(date), Arg::Plain(mail_field(&m, "sender-name")), Arg::Plain(message)]));
            }
            m["read"] = json!(true);
            kept.push(m);
        }
        if lines.is_empty() {
            set_mails(ctx.ess, u, kept);
            ctx.tl("noMail", &[]);
            return Err(CmdError::Quiet);
        }
        let text = Text { lines, mini: true, ..Text::default() };
        let label = format!("{} {}", ctx.label, args[0]);
        simple_page(ctx.ess, ctx.host, ctx.sender, &text, args.get(1).map(String::as_str), &label);
        ctx.tl("mailClear", &[]);
        set_mails(ctx.ess, u, kept);
        return Ok(());
    }
    if args.len() >= 3 && sub == "send" {
        if !ctx.has("essentials.mail.send") {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain("essentials.mail.send".into())]));
        }
        if is_muted(ctx.ess, u) {
            return Err(muted_error(ctx.ess, ctx.host, u));
        }
        let target = ctx.any_player(&args[1]).map_err(|_| never_on_server(&args[1]))?;
        let msg = format_message(ctx.ess, ctx.host, ctx.sender, "essentials.mail", &sanitize_string(&strip_format(&final_arg(args, 2))));
        if msg.chars().count() > 1000 {
            return Err(CmdError::Tl("mailTooLong", vec![]));
        }
        if !ignores(ctx.ess, ctx.host, target, u) {
            mail_rate(ctx.ess, ctx.host)?;
            send_mail(ctx.ess, ctx.host, target, me, &msg, 0);
        }
        let (d, n) = (display_name(ctx.ess, ctx.host, target), name_of(ctx.ess, ctx.host, target));
        ctx.tl("mailSentTo", &[Arg::Plain(d), Arg::Plain(n)]);
        send_legacy(ctx.ess, ctx.host, ctx.sender, &msg);
        return Ok(());
    }
    if args.len() >= 4 && sub == "sendtemp" {
        if !ctx.has("essentials.mail.sendtemp") {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain("essentials.mail.sendtemp".into())]));
        }
        if is_muted(ctx.ess, u) {
            return Err(muted_error(ctx.ess, ctx.host, u));
        }
        let target = ctx.any_player(&args[1]).map_err(|_| never_on_server(&args[1]))?;
        let expire = parse_date_diff(now, &args[2], true).ok_or(CmdError::Tl("illegalDate", vec![]))?;
        let msg = format_message(ctx.ess, ctx.host, ctx.sender, "essentials.mail", &sanitize_string(&strip_format(&final_arg(args, 3))));
        if msg.chars().count() > 1000 {
            return Err(CmdError::Tl("mailTooLong", vec![]));
        }
        if !ignores(ctx.ess, ctx.host, target, u) {
            mail_rate(ctx.ess, ctx.host)?;
            send_mail(ctx.ess, ctx.host, target, me, &msg, expire);
        }
        let (d, n) = (display_name(ctx.ess, ctx.host, target), name_of(ctx.ess, ctx.host, target));
        ctx.tl("mailSentToExpire", &[Arg::Plain(d), Arg::Plain(format_date_diff(now, expire)), Arg::Plain(n)]);
        send_legacy(ctx.ess, ctx.host, ctx.sender, &msg);
        return Ok(());
    }
    if args.len() > 1 && sub == "sendall" {
        if !ctx.has("essentials.mail.sendall") {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain("essentials.mail.sendall".into())]));
        }
        let msg = format_message(ctx.ess, ctx.host, ctx.sender, "essentials.mail", &sanitize_string(&strip_format(&final_arg(args, 1))));
        mail_send_all(ctx.ess, ctx.host, me, &msg, 0);
        ctx.tl("mailSent", &[]);
        return Ok(());
    }
    if args.len() >= 3 && sub == "sendtempall" {
        if !ctx.has("essentials.mail.sendtempall") {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain("essentials.mail.sendtempall".into())]));
        }
        let expire = parse_date_diff(now, &args[1], true).ok_or(CmdError::Tl("illegalDate", vec![]))?;
        let msg = format_message(ctx.ess, ctx.host, ctx.sender, "essentials.mail", &sanitize_string(&strip_format(&final_arg(args, 2))));
        mail_send_all(ctx.ess, ctx.host, me, &msg, expire);
        ctx.tl("mailSent", &[]);
        return Ok(());
    }
    if sub == "clear" {
        let mut target = u;
        let mut index = None;
        if args.len() > 1 {
            if let Some(n) = positive_int(&args[1]) {
                index = Some(n);
            } else if !ctx.has("essentials.mail.clear.others") {
                return Err(CmdError::Tl("noPerm", vec![Arg::Plain("essentials.mail.clear.others".into())]));
            } else {
                target = ctx.any_player(&args[1])?;
                index = args.get(2).and_then(|a| positive_int(a));
            }
        }
        return clear_mail(ctx, target, index);
    }
    if sub == "clearall" {
        if !ctx.has("essentials.mail.clearall") {
            return Err(CmdError::Tl("noPerm", vec![Arg::Plain("essentials.mail.clearall".into())]));
        }
        for p in ctx.ess.users.all() {
            set_mails(ctx.ess, p, Vec::new());
        }
        ctx.tl("mailClearedAll", &[]);
        return Ok(());
    }
    Err(CmdError::Usage(None))
}

fn mail_console(ctx: &mut Ctx, args: &[String], sub: &str) -> CmdResult {
    let now = ctx.host.now_millis();
    let console = (ctx.ess.tl("consoleName", &[]), None);
    if !args.is_empty() && sub == "read" {
        return Err(CmdError::Tl("onlyPlayers", vec![Arg::Plain(format!("{} read", ctx.label))]));
    }
    if args.len() > 1 && sub == "clear" {
        let target = ctx.any_player(&args[1])?;
        let index = args.get(2).and_then(|a| positive_int(a));
        return clear_mail(ctx, target, index);
    }
    if !args.is_empty() && sub == "clearall" {
        for p in ctx.ess.users.all() {
            set_mails(ctx.ess, p, Vec::new());
        }
        ctx.tl("mailClearedAll", &[]);
        return Ok(());
    }
    let (target_arg, expire, from) = if args.len() >= 3 && sub == "send" {
        (Some(&args[1]), 0, 2)
    } else if args.len() >= 4 && sub == "sendtemp" {
        (Some(&args[1]), parse_date_diff(now, &args[2], true).ok_or(CmdError::Tl("illegalDate", vec![]))?, 3)
    } else if args.len() >= 2 && sub == "sendall" {
        (None, 0, 1)
    } else if args.len() >= 3 && sub == "sendtempall" {
        (None, parse_date_diff(now, &args[1], true).ok_or(CmdError::Tl("illegalDate", vec![]))?, 2)
    } else if args.len() >= 2 {
        (Some(&args[0]), 0, 1)
    } else {
        return Err(CmdError::Usage(None));
    };
    let msg = replace_format(&final_arg(args, from));
    match target_arg {
        Some(name) => {
            let target = ctx.any_player(name).map_err(|_| never_on_server(name))?;
            send_mail(ctx.ess, ctx.host, target, console, &msg, expire);
        }
        None => mail_send_all(ctx.ess, ctx.host, console, &msg, expire),
    }
    ctx.tl("mailSent", &[]);
    Ok(())
}

// ===== Text files (textreader) ====================================================================

/// IText: lines (legacy text, or MiniMessage when `mini`), chapter titles and bookmarks (chapter -> line).
#[derive(Clone, Debug, Default)]
pub struct Text {
    pub lines: Vec<String>,
    pub chapters: Vec<String>,
    pub bookmarks: HashMap<String, usize>,
    pub mini: bool,
}

fn parse_text(content: &str) -> Text {
    let mut t = Text::default();
    for (n, line) in content.lines().enumerate() {
        if line.chars().count() > 1 && line.starts_with('#') {
            let titles: Vec<String> = line[1..].trim().replace(' ', "_").split(',').map(str::to_string).collect();
            t.chapters.push(replace_format(&titles[0]));
            for title in &titles {
                t.bookmarks.insert(strip_essentials_format(&title.to_lowercase()), n);
            }
        }
        t.lines.push(replace_format(line));
    }
    t
}

/// TextInput: `<name>_<player>.txt`, `<name>_<group>.txt`, then `<name>.txt` (created from the default if asked).
fn read_text(ess: &mut Ess, host: &mut dyn Host, sender: Sender, name: &str, create: bool) -> Result<Text, CmdError> {
    let dir = ess.dir.clone();
    let mut file = None;
    if let Sender::Player(u) = sender {
        let pname = name_of(ess, host, u);
        let group = ess.group(host, u);
        for who in [pname, group] {
            let f = dir.join(format!("{name}_{}.txt", sanitize_file_name(&who)));
            if f.exists() {
                file = Some(f);
                break;
            }
        }
    }
    let file = file.unwrap_or_else(|| dir.join(format!("{name}.txt")));
    match std::fs::read_to_string(&file) {
        Ok(content) => Ok(parse_text(&content)),
        Err(_) if create => {
            if let Some(default) = crate::game::ess_defaults::file(&format!("{name}.txt")) {
                let _ = std::fs::write(&file, default);
            }
            Err(CmdError::Msg(format!("File {name}.txt does not exist. Creating one for you.")))
        }
        Err(_) => Ok(Text::default()),
    }
}

/// DescParseTickFormat.ticksToDate as (days, hours, minutes).
fn tick_time(ticks: i64) -> (i64, i64, i64) {
    let ticks = ticks - 18000 + 24000;
    let days = ticks.div_euclid(24000);
    let rest = ticks - days * 24000;
    let hours = rest / 1000;
    let minutes = ((rest - hours * 1000) as f64 / (1000.0 / 60.0)).floor() as i64;
    (days, hours, minutes)
}

/// DescParseTickFormat.format12 ("6:00 AM") and format24 ("06:00").
pub fn world_time12(ticks: i64) -> String {
    let (_, h, m) = tick_time(ticks);
    let (h12, ampm) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12}:{m:02} {ampm}")
}

pub fn world_time24(ticks: i64) -> String {
    let (_, h, m) = tick_time(ticks);
    format!("{h:02}:{m:02}")
}

/// One `{KEYWORD}` or `{KEYWORD:arg}` of KeywordReplacer, None when unknown.
fn keyword(ess: &mut Ess, host: &mut dyn Host, sender: Sender, tokens: &[&str]) -> Option<String> {
    let user = sender.player();
    let now = host.now_millis();
    let v = match tokens[0] {
        "PLAYER" | "DISPLAYNAME" => user.map(|u| display_name(ess, host, u)).unwrap_or_default(),
        "USERNAME" => user.map(|u| name_of(ess, host, u)).unwrap_or_default(),
        "NICKNAME" => user.map(|u| formatted_nickname(ess, u).unwrap_or_else(|| name_of(ess, host, u))).unwrap_or_default(),
        "PREFIX" => user.map(|u| perm_affix(ess, host, u, false)).unwrap_or_default(),
        "SUFFIX" => user.map(|u| perm_affix(ess, host, u, true)).unwrap_or_default(),
        "GROUP" => user.map(|u| ess.group(host, u)).unwrap_or_default(),
        "BALANCE" => user
            .map(|u| {
                let money = ess.users.get(u).and_then(|x| x.get("money").cloned()).map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).unwrap_or_else(|| "0".into());
                format!("{}{}", cfg_str(ess, "currency-symbol", "$"), ess_text::group_digits(&money))
            })
            .unwrap_or_default(),
        "MAILS" => user.map(|u| mails(ess, u).iter().filter(|m| !mail_expired(m, now)).count().to_string()).unwrap_or_default(),
        "PLAYTIME" => user.map(|u| format_date_diff(now, now - host.statistic(u, "minecraft:custom/minecraft:play_time") * 50)).unwrap_or_default(),
        "WORLD" | "WORLDNAME" => user.and_then(|u| host.location(u)).map(|l| world_name(&l.world)).unwrap_or_default(),
        "ONLINE" => {
            let mut n = 0;
            for p in host.online() {
                if !ess.is_vanished(p) {
                    n += 1;
                }
            }
            n.to_string()
        }
        "UNIQUE" => ess_text::group_digits(&ess.users.all().len().to_string()),
        "WORLDS" => host.worlds().iter().map(|w| world_name(w)).collect::<Vec<_>>().join(", "),
        "PLAYERLIST" => {
            let mut all = Vec::new();
            let mut by_group: HashMap<String, Vec<String>> = HashMap::new();
            for p in host.online() {
                if ess.is_vanished(p) && !sender.player().is_none_or(|s| ess.has(host, Sender::Player(s), "essentials.list.hidden") || ess.has(host, Sender::Player(s), "essentials.vanish.interact")) {
                    continue;
                }
                let d = display_name(ess, host, p);
                let g = ess.group(host, p);
                by_group.entry(g).or_default().push(d.clone());
                if !ess.is_vanished(p) {
                    all.push(d);
                }
            }
            match tokens.get(1) {
                None => all.join(", "),
                Some(g) => match by_group.get(&g.to_lowercase()) {
                    Some(list) => list.join(", "),
                    None => tokens.get(2).map(|s| s.to_string()).unwrap_or_default(),
                },
            }
        }
        "TIME" => chrono::Local::now().format("%-I:%M:%S %p").to_string(),
        "DATE" => chrono::Local::now().format("%b %-d, %Y").to_string(),
        "WORLDTIME12" => user.and_then(|u| host.location(u)).map(|l| world_time12(host.world_time(&l.world))).unwrap_or_default(),
        "WORLDTIME24" => user.and_then(|u| host.location(u)).map(|l| world_time24(host.world_time(&l.world))).unwrap_or_default(),
        "WORLDDATE" => user
            .and_then(|u| host.location(u))
            .map(|l| {
                let (days, _, _) = tick_time(host.world_time(&l.world));
                let date = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch") + chrono::Duration::days(days);
                date.format("%b %-d, %Y").to_string()
            })
            .unwrap_or_default(),
        "COORDS" => user
            .and_then(|u| host.location(u))
            .map(|l| {
                let [x, y, z] = l.block();
                ess.tl("coordsKeyword", &[Arg::from(x), Arg::from(y), Arg::from(z)])
            })
            .map(|m| ess_text::plain(&ess.render(&m)))
            .unwrap_or_default(),
        "TPS" => format!("{:.2}", host.stats().tps),
        "UPTIME" => format_date_diff(now, now - host.stats().uptime_ms),
        "IP" => user.and_then(|u| host.address(u)).map(|a| format!("/{a}")).unwrap_or_default(),
        "ADDRESS" => user.and_then(|u| host.address(u)).map(|a| format!("/{a}")).unwrap_or_default(),
        "PLUGINS" => "Essentials, EssentialsChat".to_string(),
        "VERSION" => "Horizon 26.3 (EssentialsX port)".to_string(),
        _ => return None,
    };
    Some(v)
}

/// KeywordReplacer over one line.
fn replace_keywords(ess: &mut Ess, host: &mut dyn Host, sender: Sender, line: &str) -> String {
    if line.starts_with('@') {
        return line.to_string();
    }
    let mut out = line.to_string();
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(['{', '}']) else { break };
        if after.as_bytes()[close] == b'{' || close == 0 {
            rest = &after[close..];
            continue;
        }
        let inner = &after[..close];
        let tokens: Vec<&str> = inner.split(':').collect();
        if let Some(v) = keyword(ess, host, sender, &tokens) {
            out = out.replace(&format!("{{{inner}}}"), &v);
        }
        rest = &after[close + 1..];
    }
    out
}

fn keyword_text(ess: &mut Ess, host: &mut dyn Host, sender: Sender, mut text: Text) -> Text {
    let lines = std::mem::take(&mut text.lines);
    text.lines = lines.iter().map(|l| replace_keywords(ess, host, sender, l)).collect();
    text
}

fn send_line(ess: &Ess, host: &mut dyn Host, to: Sender, text: &Text, line: &str) {
    if text.mini {
        ess.send_mini(host, to, line);
    } else {
        send_legacy(ess, host, to, &format!("§r{line}"));
    }
}

fn page_title(command: &str) -> String {
    match command.split_once(' ') {
        Some((a, b)) => format!("{}: {b}", capital(a)),
        None => capital(command),
    }
}

/// SimpleTextPager.showPage (mail).
fn simple_page(ess: &mut Ess, host: &mut dyn Host, to: Sender, text: &Text, page: Option<&str>, command: &str) {
    let page = page.and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1) as usize;
    let end = text.lines.len();
    let pages = end.div_ceil(9);
    if page > pages {
        ess.send_tl(host, to, "infoUnknownChapter", &[]);
        return;
    }
    ess.send_tl(host, to, "infoPages", &[Arg::from(page), Arg::from(pages), Arg::Plain(page_title(command))]);
    for line in text.lines.iter().skip((page - 1) * 9).take(9) {
        send_line(ess, host, to, text, line);
    }
    if page < pages {
        ess.send_tl(host, to, "readNextPage", &[Arg::Plain(command.to_string()), Arg::from(page + 1)]);
    }
}

/// TextPager.showPage.
#[allow(clippy::too_many_arguments)]
fn show_page(ess: &mut Ess, host: &mut dyn Host, to: Sender, text: &Text, page_str: Option<&str>, chapter_page: Option<&str>, command: Option<&str>, one_page: bool) {
    let lines = &text.lines;
    let numeric = page_str.is_none_or(|p| p.is_empty() || p.chars().all(|c| c.is_ascii_digit()));
    if numeric {
        if lines.first().is_some_and(|l| l.starts_with('#')) {
            if one_page {
                return;
            }
            ess.send_tl(host, to, "infoChapter", &[]);
            send_legacy(ess, host, to, &text.chapters.join(", "));
            return;
        }
        let page = page_str.and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1) as usize;
        let start = if one_page { 0 } else { (page - 1) * 9 };
        let end = lines.iter().position(|l| l.starts_with('#')).unwrap_or(lines.len());
        let pages = end.div_ceil(9);
        if page > pages {
            ess.send_tl(host, to, "infoUnknownChapter", &[]);
            return;
        }
        if let (false, Some(c)) = (one_page, command) {
            ess.send_tl(host, to, "infoPages", &[Arg::from(page), Arg::from(pages), Arg::Plain(page_title(c))]);
        }
        for line in lines.iter().take(end).skip(start).take(if one_page { 20 } else { 9 }) {
            send_line(ess, host, to, text, line);
        }
        if let (false, Some(c)) = (one_page, command) {
            if page < pages {
                ess.send_tl(host, to, "readNextPage", &[Arg::Plain(c.to_string()), Arg::from(page + 1)]);
            }
        }
        return;
    }
    let chapter = page_str.unwrap_or_default();
    let chapter_page = chapter_page.and_then(|p| p.parse::<i64>().ok()).map(|p| (p - 1).max(0) as usize).unwrap_or(0);
    let Some(mark) = text.bookmarks.get(&chapter.to_lowercase()).copied() else {
        ess.send_tl(host, to, "infoUnknownChapter", &[]);
        return;
    };
    let chapter_start = mark + 1;
    let chapter_end = (chapter_start..lines.len()).find(|i| lines[*i].starts_with('#')).unwrap_or(lines.len());
    let start = chapter_start + if one_page { 0 } else { chapter_page * 9 };
    let page = chapter_page + 1;
    let pages = (chapter_end - chapter_start).div_ceil(9);
    if let (false, Some(c)) = (one_page, command) {
        ess.send_tl(host, to, "infoChapterPages", &[Arg::Plain(format!("{}: {chapter}", capital(c))), Arg::from(page), Arg::from(pages)]);
    }
    for line in lines.iter().take(chapter_end).skip(start).take(if one_page { 20 } else { 9 }) {
        send_line(ess, host, to, text, line);
    }
    if let (false, Some(c)) = (one_page, command) {
        if page < pages {
            ess.send_tl(host, to, "readNextPage", &[Arg::Plain(c.to_string()), Arg::Plain(format!("{chapter} {}", page + 1))]);
        }
    }
}

/// Commandmotd / Commandrules / Commandinfo.
fn text_command(ctx: &mut Ctx, args: &[String], file: &str) -> CmdResult {
    if let Sender::Player(u) = ctx.sender {
        set_display_nick(ctx.ess, ctx.host, u);
    }
    let text = read_text(ctx.ess, ctx.host, ctx.sender, file, true)?;
    let text = keyword_text(ctx.ess, ctx.host, ctx.sender, text);
    let label = ctx.label.clone();
    show_page(ctx.ess, ctx.host, ctx.sender, &text, args.first().map(String::as_str), args.get(1).map(String::as_str), Some(&label), false);
    Ok(())
}

fn cmd_motd(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    text_command(ctx, args, "motd")
}

fn cmd_rules(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    text_command(ctx, args, "rules")
}

fn cmd_info(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    text_command(ctx, args, "info")
}

fn cmd_customtext(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if let Sender::Player(u) = ctx.sender {
        set_display_nick(ctx.ess, ctx.host, u);
    }
    let text = read_text(ctx.ess, ctx.host, ctx.sender, "custom", true)?;
    let text = keyword_text(ctx.ess, ctx.host, ctx.sender, text);
    let label = ctx.label.clone();
    let (chapter, page) = if label.eq_ignore_ascii_case("customtext") && !args.is_empty() {
        (args[0].clone(), args.get(1).cloned())
    } else {
        (label, args.first().cloned())
    };
    show_page(ctx.ess, ctx.host, ctx.sender, &text, Some(&chapter), page.as_deref(), None, false);
    Ok(())
}

/// The plugin a command's module belongs to (its plugin.yml).
fn plugin_name(module: &str) -> String {
    match module {
        "core" => "Essentials".into(),
        other => format!("Essentials{}", capital(other)),
    }
}

/// HelpInput: the commands the player may use, by plugin, filtered by a search term.
fn help_input(ess: &mut Ess, host: &mut dyn Host, u: Uuid, search: &str) -> Text {
    let who = Sender::Player(u);
    let mut lines = Vec::new();
    let mut new_lines = Vec::new();
    if !search.is_empty() {
        lines.push(ess.tl("helpMatching", &[Arg::Plain(search.to_string())]));
    }
    let mut plugins: Vec<String> = Vec::new();
    for meta in ess_meta::COMMANDS {
        let p = plugin_name(meta.module);
        if !plugins.contains(&p) {
            plugins.push(p);
        }
    }
    let registered: Vec<&'static str> = ess.commands.keys().copied().collect();
    for plugin in plugins {
        let low = plugin.to_lowercase();
        if low == search {
            lines.clear();
            new_lines.clear();
            lines.push(ess.tl("helpFrom", &[Arg::Plain(plugin.clone())]));
        }
        let mut plugin_lines = Vec::new();
        for name in &registered {
            let Some(meta) = ess_meta::command(name) else { continue };
            if plugin_name(meta.module) != plugin {
                continue;
            }
            let desc = meta.description;
            if !search.is_empty() && !low.contains(search) && !name.to_lowercase().contains(search) && !desc.to_lowercase().contains(search) {
                continue;
            }
            if !cmd_disabled(ess, name) && ess.has(host, who, &format!("essentials.{name}")) {
                plugin_lines.push(ess.tl("helpLine", &[Arg::Plain(name.to_string()), Arg::Plain(desc.to_string())]));
            }
        }
        if !plugin_lines.is_empty() {
            new_lines.extend(plugin_lines);
            if low == search {
                break;
            }
            if search.is_empty() {
                lines.push(ess.tl("helpPlugin", &[Arg::Plain(plugin.clone()), Arg::Plain(low.clone())]));
            }
        }
    }
    lines.extend(new_lines);
    Text { lines, mini: true, ..Text::default() }
}

fn cmd_help(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(u) = ctx.sender else {
        ctx.tl("helpConsole", &[]);
        return Ok(());
    };
    let mut page = args.first().cloned();
    let mut chapter = args.get(1).cloned();
    let mut command = ctx.label.clone();
    let input = read_text(ctx.ess, ctx.host, ctx.sender, "help", false)?;
    let output = if input.lines.is_empty() {
        if let Some(cmd) = page.as_deref().and_then(|p| p.strip_prefix('/')) {
            if let Some(name) = ctx.ess.command_for(cmd) {
                let cmd = cmd.to_string();
                ctx.tl("commandHelpLine1", &[Arg::Plain(cmd.clone())]);
                let meta = ess_meta::command(name);
                let described = ctx.ess.message(&format!("{name}CommandDescription"));
                let description = if described.ends_with("CommandDescription") { meta.map_or("", |m| m.description).to_string() } else { described };
                ctx.tl("commandHelpLine2", &[Arg::Parsed(description)]);
                let aliases = meta.map(|m| m.aliases.join(", ")).unwrap_or_default();
                ctx.tl("commandHelpLine4", &[Arg::Plain(format!("[{aliases}]"))]);
                ctx.tl("commandHelpLine3", &[]);
                let usages = ctx.ess.commands.get(name).map(|c| c.usages).unwrap_or(&[]);
                if usages.is_empty() {
                    let usage = meta.map(|m| m.usage.replace("<command>", &cmd)).unwrap_or_default();
                    ctx.send_mini(&escape_tags(&usage));
                } else {
                    for (usage, key) in usages {
                        let desc = ctx.ess.tl(key, &[]);
                        ctx.tl("commandHelpLineUsage", &[Arg::Parsed(usage.replace("<command>", &cmd)), Arg::Parsed(desc)]);
                    }
                }
                return Ok(());
            }
        }
        let output = match page.as_deref() {
            None => help_input(ctx.ess, ctx.host, u, ""),
            Some(p) if p.parse::<i64>().is_ok() => help_input(ctx.ess, ctx.host, u, ""),
            Some(p) => {
                let p: String = p.chars().take(if p.chars().count() > 26 { 25 } else { 26 }).collect();
                let out = help_input(ctx.ess, ctx.host, u, &p.to_lowercase());
                command = format!("{command} {p}");
                page = chapter.take();
                out
            }
        };
        chapter = None;
        output
    } else {
        set_display_nick(ctx.ess, ctx.host, u);
        keyword_text(ctx.ess, ctx.host, ctx.sender, input)
    };
    show_page(ctx.ess, ctx.host, ctx.sender, &output, page.as_deref(), chapter.as_deref(), Some(&command), false);
    Ok(())
}

// ===== Command tree (tab completion for vanilla clients) ==========================================

/// Every Essentials label as a literal with one greedy string argument (commands.json form), for the client's
/// command tree: vanilla clients then know and tab-complete Essentials' commands.
pub fn command_tree(ess: &Ess) -> Value {
    let mut children = serde_json::Map::new();
    for label in ess.labels.keys() {
        if cmd_disabled(ess, label) {
            continue;
        }
        children.insert(
            label.clone(),
            json!({
                "type": "literal",
                "executable": true,
                "children": {"args": {"type": "argument", "parser": "brigadier:string", "properties": {"type": "greedy"}, "executable": true}},
            }),
        );
    }
    json!({"type": "root", "children": children})
}

// ===== DateUtil ===================================================================================

#[derive(Clone, Copy)]
enum Unit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

fn add_unit(d: chrono::DateTime<chrono::Utc>, unit: Unit, n: i64) -> chrono::DateTime<chrono::Utc> {
    use chrono::Duration;
    let months = |d: chrono::DateTime<chrono::Utc>, m: i64| {
        if m >= 0 { d.checked_add_months(chrono::Months::new(m as u32)) } else { d.checked_sub_months(chrono::Months::new((-m) as u32)) }.unwrap_or(d)
    };
    match unit {
        Unit::Year => months(d, n * 12),
        Unit::Month => months(d, n),
        Unit::Week => d + Duration::days(7 * n),
        Unit::Day => d + Duration::days(n),
        Unit::Hour => d + Duration::hours(n),
        Unit::Minute => d + Duration::minutes(n),
        Unit::Second => d + Duration::seconds(n),
    }
}

fn utc(ms: i64) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.timestamp_millis_opt(ms).single().unwrap_or_default()
}

/// One `(\d+)\s*<unit>[a-z]*[,\s]*` group of DateUtil's pattern at `i`; returns (value, end).
fn date_group(c: &[char], i: usize, unit: &str, optional_unit: bool) -> Option<(i64, usize)> {
    let mut j = i;
    while j < c.len() && c[j].is_ascii_digit() {
        j += 1;
    }
    if j == i {
        return None;
    }
    let value: i64 = c[i..j].iter().collect::<String>().parse().ok()?;
    let mut k = j;
    while k < c.len() && c[k].is_whitespace() {
        k += 1;
    }
    let u: Vec<char> = unit.chars().collect();
    let has_unit = c.len() >= k + u.len() && c[k..k + u.len()].iter().zip(&u).all(|(a, b)| a.to_ascii_lowercase() == *b);
    if has_unit {
        k += u.len();
        while k < c.len() && c[k].is_ascii_alphabetic() {
            k += 1;
        }
    } else if optional_unit {
        return Some((value, j));
    } else {
        return None;
    }
    while k < c.len() && (c[k] == ',' || c[k].is_whitespace()) {
        k += 1;
    }
    Some((value, k))
}

/// DateUtil.parseDateDiff: `1y2mo3w4d5h6m7s` (any order of units as the pattern allows) from `now`, capped at 10 years.
pub fn parse_date_diff(now: i64, text: &str, future: bool) -> Option<i64> {
    let c: Vec<char> = text.chars().collect();
    let units = [("y", Unit::Year), ("mo", Unit::Month), ("w", Unit::Week), ("d", Unit::Day), ("h", Unit::Hour), ("m", Unit::Minute)];
    for start in 0..c.len().max(1) {
        let mut i = start;
        let mut values: Vec<(Unit, i64)> = Vec::new();
        for (u, unit) in units {
            if let Some((v, end)) = date_group(&c, i, u, false) {
                values.push((unit, v));
                i = end;
            }
        }
        if let Some((v, end)) = date_group(&c, i, "s", true) {
            values.push((Unit::Second, v));
            i = end;
        }
        if i == start || values.is_empty() {
            continue;
        }
        let sign = if future { 1 } else { -1 };
        let mut d = utc(now);
        for (unit, v) in values {
            let v = if matches!(unit, Unit::Year) { v.min(100_000) } else { v };
            if v > 0 {
                d = add_unit(d, unit, v * sign);
            }
        }
        let max = add_unit(utc(now), Unit::Year, 10);
        return Some(if d > max { max.timestamp_millis() } else { d.timestamp_millis() });
    }
    None
}

/// DateUtil.formatDateDiff: up to three units ("1 day 2 hours 3 minutes"), or "now".
pub fn format_date_diff(now: i64, date: i64) -> String {
    let tl = |k: &str| crate::game::ess_messages::locale("en").and_then(|t| crate::game::ess_messages::lookup(t, k)).unwrap_or(k).to_string();
    if date == now {
        return tl("now");
    }
    let future = date > now;
    let mut from = utc(now);
    let to = utc(date + if future { 50 } else { -50 });
    let units = [(Unit::Year, "year"), (Unit::Month, "month"), (Unit::Day, "day"), (Unit::Hour, "hour"), (Unit::Minute, "minute"), (Unit::Second, "second")];
    let mut out = Vec::new();
    for (unit, name) in units {
        if out.len() > 2 {
            break;
        }
        let step = if future { 1 } else { -1 };
        let mut diff = 0;
        let mut saved = from;
        while (future && from <= to) || (!future && from >= to) {
            saved = from;
            from = add_unit(from, unit, step);
            diff += 1;
        }
        diff -= 1;
        from = saved;
        if diff > 0 {
            out.push(format!("{diff} {}", tl(&if diff > 1 { format!("{name}s") } else { name.to_string() })));
        }
    }
    if out.is_empty() { tl("now") } else { out.join(" ") }
}

// ===== A small java.util.regex subset (nick regex, nick blacklist, URLs) ==========================

#[derive(Clone, Debug)]
enum ClassItem {
    Range(char, char),
    Digit(bool),
    Word(bool),
    Space(bool),
}

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Class(Vec<ClassItem>, bool),
    Start,
    End,
    Group(Vec<Vec<Node>>, Option<usize>),
    Repeat(Box<Node>, usize, usize, bool),
}

/// Backtracking regex: literals, `.`, classes, `\d\w\s` (and negations), anchors, groups, `|`, greedy and lazy
/// `* + ? {n,m}`, `(?i)`. Enough for EssentialsX's nick and URL patterns and typical blacklists.
pub struct Regex {
    alts: Vec<Vec<Node>>,
    groups: usize,
    icase: bool,
}

struct Parser<'a> {
    c: &'a [char],
    i: usize,
    groups: usize,
    icase: bool,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }
    fn alts(&mut self) -> Option<Vec<Vec<Node>>> {
        let mut alts = vec![self.seq()?];
        while self.peek() == Some('|') {
            self.i += 1;
            alts.push(self.seq()?);
        }
        Some(alts)
    }
    fn seq(&mut self) -> Option<Vec<Node>> {
        let mut seq = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '|' || ch == ')' {
                break;
            }
            let Some(atom) = self.atom()? else { continue };
            let atom = self.quantified(atom)?;
            seq.push(atom);
        }
        Some(seq)
    }
    fn escape(&mut self) -> Option<Node> {
        let e = self.peek()?;
        self.i += 1;
        Some(match e {
            'd' => Node::Class(vec![ClassItem::Digit(true)], false),
            'D' => Node::Class(vec![ClassItem::Digit(false)], false),
            'w' => Node::Class(vec![ClassItem::Word(true)], false),
            'W' => Node::Class(vec![ClassItem::Word(false)], false),
            's' => Node::Class(vec![ClassItem::Space(true)], false),
            'S' => Node::Class(vec![ClassItem::Space(false)], false),
            'n' => Node::Char('\n'),
            't' => Node::Char('\t'),
            other => Node::Char(other),
        })
    }
    /// One atom; Ok(None) for a flag group.
    fn atom(&mut self) -> Option<Option<Node>> {
        let ch = self.peek()?;
        self.i += 1;
        Some(Some(match ch {
            '.' => Node::Any,
            '^' => Node::Start,
            '$' => Node::End,
            '\\' => self.escape()?,
            '[' => self.class()?,
            '(' => {
                let mut capture = true;
                if self.peek() == Some('?') {
                    self.i += 1;
                    match self.peek()? {
                        ':' => {
                            self.i += 1;
                            capture = false;
                        }
                        'i' => {
                            self.i += 1;
                            if self.peek()? != ')' {
                                return None;
                            }
                            self.i += 1;
                            self.icase = true;
                            return Some(None);
                        }
                        _ => return None,
                    }
                }
                let index = capture.then(|| {
                    self.groups += 1;
                    self.groups
                });
                let alts = self.alts()?;
                if self.peek()? != ')' {
                    return None;
                }
                self.i += 1;
                Node::Group(alts, index)
            }
            other => Node::Char(other),
        }))
    }
    fn class(&mut self) -> Option<Node> {
        let mut items = Vec::new();
        let negated = self.peek() == Some('^');
        if negated {
            self.i += 1;
        }
        let mut first = true;
        loop {
            let ch = self.peek()?;
            self.i += 1;
            if ch == ']' && !first {
                break;
            }
            first = false;
            let single = if ch == '\\' {
                match self.escape()? {
                    Node::Char(c) => Some(c),
                    Node::Class(mut v, _) => {
                        items.append(&mut v);
                        None
                    }
                    _ => None,
                }
            } else {
                Some(ch)
            };
            let Some(lo) = single else { continue };
            if self.peek() == Some('-') && self.c.get(self.i + 1).is_some_and(|c| *c != ']') {
                self.i += 1;
                let mut hi = self.peek()?;
                self.i += 1;
                if hi == '\\' {
                    hi = self.peek()?;
                    self.i += 1;
                }
                items.push(ClassItem::Range(lo, hi));
            } else {
                items.push(ClassItem::Range(lo, lo));
            }
        }
        Some(Node::Class(items, negated))
    }
    fn number(&mut self) -> Option<usize> {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        self.c[start..self.i].iter().collect::<String>().parse().ok()
    }
    fn quantified(&mut self, atom: Node) -> Option<Node> {
        let (min, max) = match self.peek() {
            Some('*') => (0, usize::MAX),
            Some('+') => (1, usize::MAX),
            Some('?') => (0, 1),
            Some('{') if self.c.get(self.i + 1).is_some_and(|c| c.is_ascii_digit()) => {
                self.i += 1;
                let min = self.number()?;
                let max = if self.peek() == Some(',') {
                    self.i += 1;
                    if self.peek() == Some('}') { usize::MAX } else { self.number()? }
                } else {
                    min
                };
                if self.peek()? != '}' {
                    return None;
                }
                (min, max)
            }
            _ => return Some(atom),
        };
        self.i += 1;
        let mut greedy = true;
        match self.peek() {
            Some('?') => {
                greedy = false;
                self.i += 1;
            }
            Some('+') => self.i += 1,
            _ => {}
        }
        Some(Node::Repeat(Box::new(atom), min, max, greedy))
    }
}

type Caps = Vec<Option<(usize, usize)>>;

struct Matcher<'a> {
    s: &'a [char],
    icase: bool,
}

impl Matcher<'_> {
    fn eq(&self, a: char, b: char) -> bool {
        a == b || (self.icase && a.to_lowercase().eq(b.to_lowercase()))
    }
    fn class_has(&self, items: &[ClassItem], c: char) -> bool {
        let one = |c: char| {
            items.iter().any(|it| match it {
                ClassItem::Range(lo, hi) => *lo <= c && c <= *hi,
                ClassItem::Digit(p) => c.is_ascii_digit() == *p,
                ClassItem::Word(p) => (c.is_ascii_alphanumeric() || c == '_') == *p,
                ClassItem::Space(p) => matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c') == *p,
            })
        };
        one(c) || (self.icase && (c.to_lowercase().any(one) || c.to_uppercase().any(one)))
    }
    fn seq(&self, nodes: &[Node], i: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
        match nodes.split_first() {
            None => k(i, caps),
            Some((n, rest)) => self.node(n, i, caps, &mut |j, c| self.seq(rest, j, c, &mut *k)),
        }
    }
    fn node(&self, n: &Node, i: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
        match n {
            Node::Char(c) => i < self.s.len() && self.eq(self.s[i], *c) && k(i + 1, caps),
            Node::Any => i < self.s.len() && self.s[i] != '\n' && k(i + 1, caps),
            Node::Class(items, neg) => i < self.s.len() && self.class_has(items, self.s[i]) != *neg && k(i + 1, caps),
            Node::Start => i == 0 && k(i, caps),
            Node::End => i == self.s.len() && k(i, caps),
            Node::Group(alts, index) => {
                for alt in alts {
                    let saved = caps.clone();
                    let ok = self.seq(alt, i, caps, &mut |j, c| match index {
                        Some(g) => {
                            let old = c[*g];
                            c[*g] = Some((i, j));
                            if k(j, c) {
                                return true;
                            }
                            c[*g] = old;
                            false
                        }
                        None => k(j, c),
                    });
                    if ok {
                        return true;
                    }
                    *caps = saved;
                }
                false
            }
            Node::Repeat(inner, min, max, greedy) => self.repeat(inner, *min, *max, *greedy, 0, i, caps, k),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn repeat(&self, n: &Node, min: usize, max: usize, greedy: bool, count: usize, i: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
        if greedy {
            if count < max && self.node(n, i, caps, &mut |j, c| j != i && self.repeat(n, min, max, greedy, count + 1, j, c, &mut *k)) {
                return true;
            }
            count >= min && k(i, caps)
        } else {
            if count >= min && k(i, caps) {
                return true;
            }
            count < max && self.node(n, i, caps, &mut |j, c| j != i && self.repeat(n, min, max, greedy, count + 1, j, c, &mut *k))
        }
    }
}

impl Regex {
    pub fn new(pattern: &str) -> Option<Regex> {
        let c: Vec<char> = pattern.chars().collect();
        let mut p = Parser { c: &c, i: 0, groups: 0, icase: false };
        let alts = p.alts()?;
        if p.i != c.len() {
            return None;
        }
        Some(Regex { alts, groups: p.groups, icase: p.icase })
    }

    /// Captures of the first match at or after `from` (index 0 = the whole match).
    fn find_from(&self, s: &[char], from: usize) -> Option<Caps> {
        let m = Matcher { s, icase: self.icase };
        let root = Node::Group(self.alts.clone(), Some(0));
        for start in from..=s.len() {
            let mut caps: Caps = vec![None; self.groups + 1];
            let mut found = None;
            if m.node(&root, start, &mut caps, &mut |_, c| {
                found = Some(c.clone());
                true
            }) {
                return found;
            }
        }
        None
    }

    /// Matcher.matches: the whole string.
    pub fn is_match(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        let m = Matcher { s: &chars, icase: self.icase };
        let root = Node::Group(self.alts.clone(), None);
        let mut caps: Caps = vec![None; self.groups + 1];
        m.node(&root, 0, &mut caps, &mut |j, _| j == chars.len())
    }

    /// Matcher.find: anywhere in the string.
    pub fn find_in(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        self.find_from(&chars, 0).is_some()
    }

    /// Matcher.replaceAll with a function of the groups (missing groups are empty).
    pub fn replace_all(&self, s: &str, f: impl Fn(&[String]) -> String) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let mut at = 0;
        while at <= chars.len() {
            let Some(caps) = self.find_from(&chars, at) else { break };
            let (start, end) = caps[0].expect("whole match");
            out.extend(&chars[at..start]);
            let groups: Vec<String> = caps.iter().map(|g| g.map(|(a, b)| chars[a..b].iter().collect()).unwrap_or_default()).collect();
            out.push_str(&f(&groups));
            if end == start {
                if let Some(ch) = chars.get(end) {
                    out.push(*ch);
                }
                at = end + 1;
            } else {
                at = end;
            }
        }
        if at < chars.len() {
            out.extend(&chars[at..]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    // L1 tests of the social area (included by ess_social.rs as its `tests` module).
    use super::*;
    use crate::game::ess_testkit::*;

    /// A server with these config lines; players joining through `j` have their join lines drained.
    fn srv(config: &str) -> TestServer {
        let mut t = TestServer::with_config(config);
        // The Spawn module's first-join welcome (tp area) is broadcast two ticks after a join; these tests
        // count lines, so they run without it.
        crate::game::ess_yaml::set(&mut t.ess.config, "newbies.announce-format", serde_json::json!(""));
        t
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
        let mut t = srv("auto-afk: 0");
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
