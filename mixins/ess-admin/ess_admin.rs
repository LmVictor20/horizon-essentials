// horizon-essentials area `admin` (pure logic, standalone-tested): moderation and administration, ported from
// EssentialsX (GPL-3.0): ban*, tempban*, unban*, kick*, jails (Jails.java), sudo, kill, seen, whois, vanish, invsee,
// gc, essentials, backup, list (PlayerList.java), near, getpos, compass, depth, playtime; plus ban enforcement on
// join, jail restrictions, command cooldowns and console command logging (EssentialsPlayerListener), DateUtil.

use crate::game::ess_core::*;
use crate::game::ess_text::{escape_tags, legacy_to_mini, plain, strip_legacy};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The vanilla statistic EssentialsX calls PLAY_ONE_TICK (ticks played).
pub const PLAY_TIME: &str = "minecraft:custom/minecraft:play_time";
/// Console.NAME / Console.displayName().
const CONSOLE_DISPLAY: &str = "Console";
/// The console's CommandSender.getName().
const CONSOLE_SENDER: &str = "CONSOLE";
const ESS_VERSION: &str = "2.21.0-horizon";

/// This area's in-memory state.
#[derive(Default)]
pub struct AdminState {
    /// jail.yml, lower-case names; loaded on first use and by /essentials reload.
    jails: Option<BTreeMap<String, Loc>>,
    /// When each vanished player vanished (User.lastVanishTime).
    vanished_at: HashMap<Uuid, i64>,
    /// Cooldowns not saved to userdata (command-cooldown-persistence: false): (pattern, expiry).
    cooldowns: HashMap<Uuid, Vec<(String, i64)>>,
    /// viewer -> inventory owner of an open /invsee.
    invsee: HashMap<Uuid, Uuid>,
    pub debug: bool,
    /// Set by the backup process when it ends.
    backup: Option<Arc<AtomicBool>>,
}

fn st(ess: &mut Ess) -> &mut AdminState {
    ess.state::<AdminState>()
}

fn command(name: &'static str, run: CmdFn, usages: &'static [(&'static str, &'static str)]) -> Command {
    Command { name, run, tab: Some(tab_players), usages }
}

pub fn register(ess: &mut Ess) {
    let commands = [
        command("backup", cmd_backup, &[]),
        command("ban", cmd_ban, &[("/<command> <player> [reason]", "banCommandUsage1Description")]),
        command("banip", cmd_banip, &[("/<command> <address> [reason]", "banipCommandUsage1Description")]),
        command("compass", cmd_compass, &[]),
        command("deljail", cmd_deljail, &[("/<command> <jailname>", "deljailCommandUsage1Description")]),
        command("depth", cmd_depth, &[]),
        command(
            "essentials",
            cmd_essentials,
            &[
                ("/<command> reload", "essentialsCommandUsage1Description"),
                ("/<command> version", "essentialsCommandUsage2Description"),
                ("/<command> commands", "essentialsCommandUsage3Description"),
                ("/<command> debug", "essentialsCommandUsage4Description"),
                ("/<command> cleanup", "essentialsCommandUsage6Description"),
                ("/<command> homes", "essentialsCommandUsage7Description"),
            ],
        ),
        command("gc", cmd_gc, &[]),
        command("getpos", cmd_getpos, &[("/<command> [player]", "getposCommandUsage1Description")]),
        command("invsee", cmd_invsee, &[("/<command> <player>", "invseeCommandUsage1Description")]),
        command("jailedplayers", cmd_jailedplayers, &[]),
        command("jails", cmd_jails, &[]),
        command("kick", cmd_kick, &[("/<command> <player> [reason]", "kickCommandUsage1Description")]),
        command("kickall", cmd_kickall, &[("/<command> [reason]", "kickallCommandUsage1Description")]),
        command("kill", cmd_kill, &[("/<command> <player>", "killCommandUsage1Description")]),
        command("list", cmd_list, &[("/<command> [group]", "listCommandUsage1Description")]),
        command(
            "near",
            cmd_near,
            &[
                ("/<command>", "nearCommandUsage1Description"),
                ("/<command> <radius>", "nearCommandUsage2Description"),
                ("/<command> <player>", "nearCommandUsage3Description"),
                ("/<command> <player> <radius>", "nearCommandUsage4Description"),
            ],
        ),
        command("playtime", cmd_playtime, &[("/<command>", "playtimeCommandUsage1Description"), ("/<command> <player>", "playtimeCommandUsage2Description")]),
        command("seen", cmd_seen, &[("/<command> <playername>", "seenCommandUsage1Description")]),
        command("setjail", cmd_setjail, &[("/<command> <jailname>", "setjailCommandUsage1Description")]),
        command("sudo", cmd_sudo, &[("/<command> <player> <command> [args]", "sudoCommandUsage1Description")]),
        command("tempban", cmd_tempban, &[("/<command> <player> <datediff> [reason]", "tempbanCommandUsage1Description")]),
        command("tempbanip", cmd_tempbanip, &[("/<command> <player|ip-address> <datediff> [reason]", "tempbanipCommandUsage1Description")]),
        command("togglejail", cmd_togglejail, &[]),
        command("unban", cmd_unban, &[("/<command> <player>", "unbanCommandUsage1Description")]),
        command("unbanip", cmd_unbanip, &[("/<command> <address>", "unbanipCommandUsage1Description")]),
        command("vanish", cmd_vanish, &[("/<command> [player]", "vanishCommandUsage1Description")]),
        command("whois", cmd_whois, &[("/<command> <player>", "whoisCommandUsage1Description")]),
    ];
    for c in commands {
        ess.register(c);
    }
    ess.listeners.join.push(on_join);
    ess.listeners.quit.push(on_quit);
    ess.listeners.tick.push(on_tick);
    ess.listeners.command.push(on_command);
    ess.listeners.block_break.push(on_block_break);
    ess.listeners.block_place.push(on_block_place);
    ess.listeners.interact.push(on_interact);
    ess.listeners.respawn.push(on_respawn);
    ess.listeners.container_click.push(on_container_click);
    ess.listeners.container_close.push(on_container_close);
}

/// Tab completion: online player names the sender can see for the first argument, jails for the second of /jail.
fn tab_players(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    if ctx.name == "togglejail" && args.len() == 2 || ctx.name == "deljail" && args.len() == 1 {
        return jails(ctx.ess).keys().cloned().collect();
    }
    if args.len() != 1 || matches!(ctx.name, "backup" | "compass" | "depth" | "gc" | "jails" | "jailedplayers" | "kickall" | "setjail" | "essentials") {
        return Vec::new();
    }
    let mut out = Vec::new();
    for p in ctx.host.online() {
        if ctx.ess.can_see(ctx.host, ctx.sender, p) {
            out.extend(ctx.host.name(p));
        }
    }
    out
}

// ===== Small helpers ==============================================================================

fn has_node(ess: &mut Ess, host: &mut dyn Host, player: Uuid, node: &str) -> bool {
    ess.has(host, Sender::Player(player), node)
}

fn is_online(host: &mut dyn Host, player: Uuid) -> bool {
    host.online().contains(&player)
}

fn name_of(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    host.name(player).or_else(|| ess.users.get(player).and_then(|u| u.name())).unwrap_or_else(|| uuid_string(player))
}

/// player.getDisplayName(): the nickname (with nickname-prefix) when one is set, else the name. Legacy codes kept.
fn display_name(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    let name = name_of(ess, host, player);
    let nick = ess.users.get(player).and_then(|u| u.str("nickname")).filter(|n| !n.is_empty() && *n != name);
    match nick {
        Some(n) => format!("{}{}", ess.config_str("nickname-prefix").unwrap_or_else(|| "~".into()), replace_format(&n)),
        None => name,
    }
}

fn sender_display(ctx: &mut Ctx) -> String {
    match ctx.sender {
        Sender::Player(p) => display_name(ctx.ess, ctx.host, p),
        Sender::Console => CONSOLE_DISPLAY.into(),
    }
}

/// sender.getSender().getName().
fn sender_name(ctx: &mut Ctx) -> String {
    match ctx.sender {
        Sender::Player(p) => name_of(ctx.ess, ctx.host, p),
        Sender::Console => CONSOLE_SENDER.into(),
    }
}

fn user_bool(ess: &mut Ess, player: Uuid, key: &str) -> bool {
    ess.users.get(player).is_some_and(|u| u.bool(key))
}

fn user_i64(ess: &mut Ess, player: Uuid, key: &str) -> i64 {
    ess.users.get(player).and_then(|u| u.i64(key).or_else(|| u.f64(key).map(|f| f as i64))).unwrap_or(0)
}

fn user_str(ess: &mut Ess, player: Uuid, key: &str) -> Option<String> {
    ess.users.get(player).and_then(|u| u.str(key))
}

/// EssentialsX's LOGGER.info(miniToLegacy(tlLiteral(key, args))): one log line per message line.
fn log_tl(ess: &Ess, host: &mut dyn Host, key: &str, args: &[Arg]) {
    let text = ess.tl(key, args);
    for line in text.split('\n') {
        host.log(&ess.render(line));
    }
}

fn log_plain(ess: &Ess, host: &mut dyn Host, line: &str) {
    host.log(&ess.render(&escape_tags(line)));
}

/// ess.broadcastTl(null, node, key, args): online users holding `node`.
fn notify(ess: &mut Ess, host: &mut dyn Host, node: &str, key: &str, args: &[Arg]) {
    for p in host.online() {
        if ess.has(host, Sender::Player(p), node) {
            ess.send_tl(host, Sender::Player(p), key, args);
        }
    }
}

/// FormatUtil.replaceFormat: `&` color and format codes as `§` codes.
pub fn replace_format(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if *c == '&' && chars.get(i + 1).is_some_and(|n| "0123456789abcdefklmnorABCDEFKLMNOR".contains(*n)) {
            out.push('§');
        } else {
            out.push(*c);
        }
    }
    out
}

/// A kick/ban reason as typed: `\n` and `|` start new lines, `&` codes become colors.
fn reason_text(s: &str) -> String {
    replace_format(&s.replace("\\n", "\n").replace('|', "\n"))
}

/// Legacy text (with `§` codes) as MiniMessage.
fn legacy_mini(s: &str) -> String {
    legacy_to_mini(&escape_tags(s), false)
}

fn kick_mini(ess: &Ess, host: &mut dyn Host, player: Uuid, mini: &str) {
    let spans = ess.render(mini);
    host.kick(player, &spans);
}

/// FormatUtil.validIP: a dotted IPv4 address.
pub fn valid_ip(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u32>().is_ok_and(|n| n <= 255))
}

/// Bukkit's world name for a dimension id (`world`, `world_nether`, `world_the_end`).
pub fn world_name(world: &str) -> String {
    match world {
        "minecraft:overworld" => "world".into(),
        "minecraft:the_nether" => "world_nether".into(),
        "minecraft:the_end" => "world_the_end".into(),
        other => other.strip_prefix("minecraft:").unwrap_or(other).to_string(),
    }
}

/// java.text.DecimalFormat with grouping and at most `decimals` fraction digits (HALF_UP).
pub fn format_number(value: f64, decimals: usize) -> String {
    let factor = 10f64.powi(decimals as i32);
    let rounded = (value.abs() * factor + 0.5 + 1e-9).floor() / factor;
    let mut s = format!("{rounded:.decimals$}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    let s = crate::game::ess_text::group_digits(&s);
    if value < 0.0 && rounded != 0.0 { format!("-{s}") } else { s }
}

fn tl_bool(ess: &Ess, value: bool) -> Arg {
    Arg::Parsed(ess.tl(if value { "true" } else { "false" }, &[]))
}

/// Essentials' getPlayer: `hidden` also finds players the sender can't see, `offline` known offline players.
fn find(ctx: &mut Ctx, search: &str, hidden: bool, offline: bool) -> Result<Uuid, CmdError> {
    let viewer = if hidden { Sender::Console } else { ctx.sender };
    ctx.ess.match_player(ctx.host, viewer, search, offline).ok_or(CmdError::PlayerNotFound)
}

/// user.canInteractVanished(); the console always can.
fn can_interact_vanished(ess: &mut Ess, host: &mut dyn Host, who: Sender) -> bool {
    ess.has(host, who, "essentials.vanish.interact")
}

/// Hidden from `viewer` (vanished and not seen by them).
fn hidden_from(ess: &mut Ess, host: &mut dyn Host, viewer: Sender, target: Uuid) -> bool {
    !ess.can_see(host, viewer, target)
}

/// Bukkit's Server.matchPlayer: an exact name, else every name containing the text.
fn match_players(host: &mut dyn Host, search: &str) -> Vec<Uuid> {
    let lower = search.to_lowercase();
    let mut out = Vec::new();
    for p in host.online() {
        let name = host.name(p).unwrap_or_default();
        if name.eq_ignore_ascii_case(search) {
            return vec![p];
        }
        if name.to_lowercase().contains(&lower) {
            out.push(p);
        }
    }
    out
}

/// EssentialsLoopCommand.loopOnlinePlayers: the players a search term names.
fn loop_players(ctx: &mut Ctx, multiple_matches: bool, wildcards: bool, term: &str) -> Result<Vec<Uuid>, CmdError> {
    if term.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    if let Sender::Player(me) = ctx.sender {
        if term == "@s" || term == "@p" {
            return Ok(vec![me]);
        }
    }
    let skip_hidden = ctx.is_player() && !can_interact_vanished(ctx.ess, ctx.host, ctx.sender);
    let mut out = Vec::new();
    if wildcards && (term == "*" || term == "**") {
        for p in ctx.host.online() {
            if !(skip_hidden && hidden_from(ctx.ess, ctx.host, ctx.sender, p)) {
                out.push(p);
            }
        }
        return Ok(out);
    }
    if multiple_matches {
        if term.trim().chars().count() < 2 {
            return Err(CmdError::PlayerNotFound);
        }
        let mut matched = match_players(ctx.host, term);
        if matched.is_empty() {
            let lower = term.to_lowercase();
            for p in ctx.host.online() {
                let display = display_name(ctx.ess, ctx.host, p);
                if strip_legacy(&display, false).to_lowercase().contains(&lower) {
                    matched.push(p);
                }
            }
        }
        for p in matched {
            if !(skip_hidden && hidden_from(ctx.ess, ctx.host, ctx.sender, p)) {
                out.push(p);
            }
        }
        if out.is_empty() {
            return Err(CmdError::PlayerNotFound);
        }
        return Ok(out);
    }
    Ok(vec![find(ctx, term, !skip_hidden, false)?])
}

// ===== DateUtil ===================================================================================

#[derive(Clone, Copy)]
enum Unit {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

fn add_unit(millis: i64, unit: Unit, n: i64) -> i64 {
    use chrono::{DateTime, Months};
    let months = |m: i64| -> i64 {
        let Some(t) = DateTime::from_timestamp_millis(millis) else { return millis };
        let moved = if m >= 0 { t.checked_add_months(Months::new(m as u32)) } else { t.checked_sub_months(Months::new((-m) as u32)) };
        moved.map_or(millis, |d| d.timestamp_millis())
    };
    match unit {
        Unit::Year => months(n.clamp(-100_000, 100_000) * 12),
        Unit::Month => months(n),
        Unit::Day => millis + n * 86_400_000,
        Unit::Hour => millis + n * 3_600_000,
        Unit::Minute => millis + n * 60_000,
        Unit::Second => millis + n * 1_000,
    }
}

/// One match of DateUtil's timePattern at `start`: the seven numbers (y, mo, w, d, h, m, s) and the end.
fn match_time(chars: &[char], start: usize) -> ([Option<i64>; 7], usize) {
    let mut groups = [None; 7];
    let mut pos = start;
    let lower = |i: usize| chars.get(i).map(|c| c.to_ascii_lowercase());
    for (g, unit) in ["y", "mo", "w", "d", "h", "m", "s"].iter().enumerate() {
        let mut p = pos;
        while chars.get(p).is_some_and(|c| c.is_ascii_digit()) {
            p += 1;
        }
        if p == pos {
            continue;
        }
        let digits: String = chars[pos..p].iter().collect();
        while chars.get(p).is_some_and(|c| c.is_whitespace()) {
            p += 1;
        }
        if g == 6 {
            // Seconds: the unit is optional.
            if lower(p) == Some('s') {
                p += 1;
                while lower(p).is_some_and(|c| c.is_ascii_lowercase()) {
                    p += 1;
                }
            }
        } else {
            let ok = unit.chars().enumerate().all(|(k, u)| lower(p + k) == Some(u));
            if !ok {
                continue;
            }
            p += unit.len();
            while lower(p).is_some_and(|c| c.is_ascii_lowercase()) {
                p += 1;
            }
            while chars.get(p).is_some_and(|c| *c == ',' || c.is_whitespace()) {
                p += 1;
            }
        }
        groups[g] = Some(digits.parse::<i64>().unwrap_or(i64::from(i32::MAX)).min(i64::from(i32::MAX)));
        pos = p;
    }
    (groups, pos)
}

/// DateUtil.parseDateDiff: `1y2mo3w4d5h6m7s` from now (or from 0 with `empty_epoch`); None = illegalDate.
pub fn parse_date_diff(time: &str, future: bool, empty_epoch: bool, now: i64) -> Option<i64> {
    let chars: Vec<char> = time.chars().collect();
    let mut found = None;
    for start in 0..=chars.len() {
        let (groups, end) = match_time(&chars, start);
        if end > start && groups.iter().any(Option::is_some) {
            found = Some(groups);
            break;
        }
    }
    let g = found?;
    let sign = if future { 1 } else { -1 };
    let mut c = if empty_epoch { 0 } else { now };
    let units = [(0, Unit::Year), (1, Unit::Month), (3, Unit::Day), (4, Unit::Hour), (5, Unit::Minute), (6, Unit::Second)];
    for (i, unit) in units {
        if i == 3 {
            if let Some(w) = g[2].filter(|w| *w > 0) {
                c = add_unit(c, Unit::Day, 7 * w * sign);
            }
        }
        if let Some(n) = g[i].filter(|n| *n > 0) {
            c = add_unit(c, unit, n * sign);
        }
    }
    let max = add_unit(now, Unit::Year, 10);
    Some(c.min(max))
}

/// DateUtil.removeTimePattern: the reason left after a leading date diff.
pub fn remove_time_pattern(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let (_, end) = match_time(&chars, 0);
    chars[end..].iter().collect::<String>().trim().to_string()
}

fn date_diff(unit: Unit, from: &mut i64, to: i64, future: bool) -> i64 {
    let mut diff = 0;
    let mut saved = *from;
    while (future && *from <= to) || (!future && *from >= to) {
        saved = *from;
        *from = add_unit(*from, unit, if future { 1 } else { -1 });
        diff += 1;
    }
    *from = saved;
    diff - 1
}

/// DateUtil.formatDateDiff(date) seen from `now`: "1 day 2 hours", at most three units, or "now".
pub fn format_date_diff(ess: &Ess, now: i64, date: i64) -> String {
    format_date_diff_with(&|key| ess.tl(key, &[]), now, date)
}

/// format_date_diff with the unit names from `tl` (the login gate runs without Essentials at hand).
pub fn format_date_diff_with(tl: &dyn Fn(&str) -> String, now: i64, date: i64) -> String {
    if date == now {
        return tl("now");
    }
    let future = date > now;
    let to = date + if future { 50 } else { -50 };
    let mut from = now;
    let mut out = String::new();
    let mut accuracy = 0;
    for (unit, one, many) in [
        (Unit::Year, "year", "years"),
        (Unit::Month, "month", "months"),
        (Unit::Day, "day", "days"),
        (Unit::Hour, "hour", "hours"),
        (Unit::Minute, "minute", "minutes"),
        (Unit::Second, "second", "seconds"),
    ] {
        if accuracy > 2 {
            break;
        }
        let diff = date_diff(unit, &mut from, to, future);
        if diff > 0 {
            accuracy += 1;
            out.push_str(&format!(" {diff} {}", tl(if diff > 1 { many } else { one })));
        }
    }
    if out.is_empty() { tl("now") } else { out.trim().to_string() }
}

fn date_diff_from_now(ctx: &mut Ctx, date: i64) -> String {
    let now = ctx.host.now_millis();
    format_date_diff(ctx.ess, now, date)
}

/// DateUtil.formatDate: java's LONG date and time in English (UTC).
pub fn format_date(millis: i64) -> String {
    chrono::DateTime::from_timestamp_millis(millis).map_or_else(String::new, |d| d.format("%B %-d, %Y at %-I:%M:%S %p UTC").to_string())
}

// ===== A small java.util.regex for command-cooldowns ============================================

#[derive(Clone, Debug)]
enum Re {
    Lit(char),
    Any,
    Class(bool, Vec<(char, char)>),
    Group(Vec<Vec<Re>>),
    Rep(Box<Re>, usize, usize),
    Start,
    End,
}

struct ReParser {
    c: Vec<char>,
    i: usize,
}

fn class_escape(c: char) -> Option<Vec<(char, char)>> {
    match c {
        'd' => Some(vec![('0', '9')]),
        'w' => Some(vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')]),
        's' => Some(vec![(' ', ' '), ('\t', '\r')]),
        _ => None,
    }
}

impl ReParser {
    fn alternatives(&mut self) -> Option<Vec<Vec<Re>>> {
        let mut alts = vec![self.sequence()?];
        while self.c.get(self.i) == Some(&'|') {
            self.i += 1;
            alts.push(self.sequence()?);
        }
        Some(alts)
    }
    fn sequence(&mut self) -> Option<Vec<Re>> {
        let mut seq = Vec::new();
        while let Some(&c) = self.c.get(self.i) {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom()?;
            let (min, max) = match self.c.get(self.i) {
                Some('*') => (0, usize::MAX),
                Some('+') => (1, usize::MAX),
                Some('?') => (0, 1),
                Some('{') => {
                    let close = self.c[self.i..].iter().position(|c| *c == '}')? + self.i;
                    let inner: String = self.c[self.i + 1..close].iter().collect();
                    let (a, b) = match inner.split_once(',') {
                        Some((a, "")) => (a.trim().parse().ok()?, usize::MAX),
                        Some((a, b)) => (a.trim().parse().ok()?, b.trim().parse().ok()?),
                        None => {
                            let n = inner.trim().parse().ok()?;
                            (n, n)
                        }
                    };
                    self.i = close;
                    (a, b)
                }
                _ => {
                    seq.push(atom);
                    continue;
                }
            };
            self.i += 1;
            if matches!(self.c.get(self.i), Some('?') | Some('+')) {
                self.i += 1;
            }
            seq.push(Re::Rep(Box::new(atom), min, max));
        }
        Some(seq)
    }
    fn atom(&mut self) -> Option<Re> {
        let c = *self.c.get(self.i)?;
        self.i += 1;
        Some(match c {
            '(' => {
                if self.c.get(self.i) == Some(&'?') && self.c.get(self.i + 1) == Some(&':') {
                    self.i += 2;
                }
                let alts = self.alternatives()?;
                if self.c.get(self.i) != Some(&')') {
                    return None;
                }
                self.i += 1;
                Re::Group(alts)
            }
            '[' => {
                let negated = self.c.get(self.i) == Some(&'^');
                if negated {
                    self.i += 1;
                }
                let mut ranges = Vec::new();
                let mut first = true;
                loop {
                    let c = *self.c.get(self.i)?;
                    self.i += 1;
                    if c == ']' && !first {
                        break;
                    }
                    first = false;
                    let lo = if c == '\\' {
                        let e = *self.c.get(self.i)?;
                        self.i += 1;
                        if let Some(r) = class_escape(e) {
                            ranges.extend(r);
                            continue;
                        }
                        e
                    } else {
                        c
                    };
                    if self.c.get(self.i) == Some(&'-') && self.c.get(self.i + 1).is_some_and(|n| *n != ']') {
                        let hi = self.c[self.i + 1];
                        self.i += 2;
                        ranges.push((lo, hi));
                    } else {
                        ranges.push((lo, lo));
                    }
                }
                Re::Class(negated, ranges)
            }
            '.' => Re::Any,
            '^' => Re::Start,
            '$' => Re::End,
            '\\' => {
                let e = *self.c.get(self.i)?;
                self.i += 1;
                match class_escape(e.to_ascii_lowercase()) {
                    Some(r) => Re::Class(e.is_ascii_uppercase(), r),
                    None => Re::Lit(e),
                }
            }
            other => Re::Lit(other),
        })
    }
}

fn re_seq(seq: &[Re], pos: usize, t: &[char], k: &dyn Fn(usize) -> bool) -> bool {
    match seq.split_first() {
        None => k(pos),
        Some((first, rest)) => re_node(first, pos, t, &|p| re_seq(rest, p, t, k)),
    }
}

fn re_node(n: &Re, pos: usize, t: &[char], k: &dyn Fn(usize) -> bool) -> bool {
    match n {
        Re::Lit(c) => t.get(pos) == Some(c) && k(pos + 1),
        Re::Any => pos < t.len() && t[pos] != '\n' && k(pos + 1),
        Re::Class(negated, ranges) => t.get(pos).is_some_and(|c| ranges.iter().any(|(a, b)| a <= c && c <= b) != *negated) && k(pos + 1),
        Re::Group(alts) => alts.iter().any(|alt| re_seq(alt, pos, t, k)),
        Re::Rep(inner, min, max) => re_rep(inner, *min, *max, 0, pos, t, k),
        Re::Start => pos == 0 && k(pos),
        Re::End => pos == t.len() && k(pos),
    }
}

fn re_rep(inner: &Re, min: usize, max: usize, count: usize, pos: usize, t: &[char], k: &dyn Fn(usize) -> bool) -> bool {
    if count < max && re_node(inner, pos, t, &|p| p != pos && re_rep(inner, min, max, count + 1, p, t, k)) {
        return true;
    }
    count >= min && k(pos)
}

/// Pattern.compile(pattern).matcher(text).matches(), for the regex subset cooldown entries use.
pub fn cooldown_matches(pattern: &str, text: &str) -> bool {
    let mut parser = ReParser { c: pattern.chars().collect(), i: 0 };
    let Some(alts) = parser.alternatives() else { return false };
    if parser.i != parser.c.len() {
        return false;
    }
    let t: Vec<char> = text.chars().collect();
    re_node(&Re::Group(alts), 0, &t, &|p| p == t.len())
}

/// Settings.getCommandCooldowns: (pattern, milliseconds) in config order.
fn cooldown_entries(ess: &Ess) -> Option<Vec<(String, i64)>> {
    let Some(Value::Object(map)) = ess.config("command-cooldowns") else { return None };
    let mut out = Vec::new();
    for (key, value) in map {
        let pattern = if let Some(re) = key.strip_prefix('^') {
            re.to_string()
        } else {
            let key = if key.starts_with("\\^") { &key[1..] } else { key.as_str() };
            format!("{}( .*)?", key.replace('*', ".*"))
        };
        let seconds = match value {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        if let Some(seconds) = seconds {
            out.push((pattern, (seconds as i64) * 1000));
        }
    }
    Some(out)
}

// ===== Events =====================================================================================

fn on_join(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _first: bool) {
    // Bans (EssentialsPlayerListener's login listener): refused with EssentialsX's messages.
    let name = name_of(ess, host, player);
    if let Some(ban) = active_ban(host, false, &name) {
        let now = host.now_millis();
        let text = match ban.expires {
            Some(e) => ess.tl("tempbanJoin", &[Arg::Plain(format_date_diff(ess, now, e)), Arg::Plain(ban.reason.clone())]),
            None => ess.tl("banJoin", &[Arg::Plain(ban.reason.clone())]),
        };
        kick_mini(ess, host, player, &text);
        return;
    }
    if let Some(ip) = host.address(player) {
        if let Some(ban) = active_ban(host, true, &ip) {
            let now = host.now_millis();
            let text = match ban.expires {
                Some(e) => ess.tl("tempbanIpJoin", &[Arg::Plain(format_date_diff(ess, now, e)), Arg::Plain(ban.reason.clone())]),
                None => ess.tl("banIpJoin", &[Arg::Plain(ban.reason.clone())]),
            };
            kick_mini(ess, host, player, &text);
            return;
        }
    }
    // Vanished players stay hidden from the newcomer (unless they may see them).
    for other in host.online() {
        if other != player && ess.is_vanished(other) {
            let seers = seers(ess, host);
            host.set_hidden(other, true, &seers);
        }
    }
    if ess.config_bool("allow-silent-join-quit", false) && has_node(ess, host, player, "essentials.silentjoin.vanish") {
        set_vanished(ess, host, player, true);
    }
    // Jails.onJailPlayerJoin.
    let now = host.now_millis();
    check_jail_timeout(ess, host, player, now);
    if user_bool(ess, player, "jailed") {
        if let Some(jail) = user_str(ess, player, "jail").filter(|j| !j.is_empty()) {
            if send_to_jail(ess, host, player, &jail) {
                ess.send_tl(host, Sender::Player(player), "jailMessage", &[]);
            } else {
                log_tl(ess, host, "returnPlayerToJailError", &[Arg::Plain(name), Arg::Parsed(ess.tl("jailNotExist", &[]))]);
            }
        }
    }
}

fn on_quit(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    if ess.is_vanished(player) {
        set_vanished(ess, host, player, false);
    }
    let viewers: Vec<Uuid> = st(ess).invsee.iter().filter(|(_, t)| **t == player).map(|(v, _)| *v).collect();
    for v in viewers {
        host.close_container(v);
        st(ess).invsee.remove(&v);
    }
    st(ess).invsee.remove(&player);
}

fn on_tick(ess: &mut Ess, host: &mut dyn Host) {
    // EssentialsTimer: jail timeouts once a second.
    if ess.scheduler.tick % 20 == 0 {
        let now = host.now_millis();
        for p in host.online() {
            if user_bool(ess, p, "jailed") {
                check_jail_timeout(ess, host, p, now);
            }
        }
    }
    // Backup: the end of a running backup, and the periodic one.
    if let Some(done) = st(ess).backup.clone() {
        if done.load(Ordering::SeqCst) {
            st(ess).backup = None;
            host.console_command("save-on");
            log_tl(ess, host, "backupFinished", &[]);
        }
    }
    let interval = ess.config_i64("backup.interval", 30) * 1200;
    if interval >= 1200 && ess.scheduler.tick % interval as u64 == 0 && (!host.online().is_empty() || ess.config_bool("backup.always-run", false)) {
        run_backup(ess, host);
    }
}

/// Before every Essentials command: console logging, cooldowns, then jail restrictions.
fn on_command(ess: &mut Ess, host: &mut dyn Host, sender: Sender, line: &str) -> Allow {
    let label = line.split_whitespace().next().unwrap_or("").to_lowercase();
    let args = line.split_once(char::is_whitespace).map(|(_, a)| a.trim()).unwrap_or("");
    let Sender::Player(player) = sender else {
        if ess.config_bool("log-console-commands", true) {
            log_plain(ess, host, &format!("{CONSOLE_SENDER} issued server command: /{label} {args}"));
        }
        return Allow::Yes;
    };
    let name = ess.command_for(&label).unwrap_or("");
    if check_cooldown(ess, host, player, name, args) == Allow::No {
        return Allow::No;
    }
    if user_bool(ess, player, "jailed") && !has_node(ess, host, player, &format!("essentials.jail.allow.{name}")) {
        if user_i64(ess, player, "timestamps.jail") > 0 {
            let pname = name_of(ess, host, player);
            let time = formatted_jail_time(ess, host, player);
            ess.send_tl(host, sender, "playerJailedFor", &[Arg::Plain(pname), Arg::Plain(time)]);
        } else {
            ess.send_tl(host, sender, "jailMessage", &[]);
        }
        return Allow::No;
    }
    Allow::Yes
}

fn check_cooldown(ess: &mut Ess, host: &mut dyn Host, player: Uuid, name: &str, args: &str) -> Allow {
    let Some(entries) = cooldown_entries(ess) else { return Allow::Yes };
    if has_node(ess, host, player, "essentials.commandcooldowns.bypass") || (!name.is_empty() && has_node(ess, host, player, &format!("essentials.commandcooldowns.bypass.{name}"))) {
        return Allow::Yes;
    }
    let full = if args.is_empty() { name.to_string() } else { format!("{name} {args}") };
    let now = host.now_millis();
    let mut saved: Vec<(String, i64)> = match ess.users.user(player).get("timestamps.command-cooldowns") {
        Some(Value::Array(a)) => a.iter().filter_map(|c| Some((c.get("pattern")?.as_str()?.to_string(), c.get("value")?.as_i64()?))).collect(),
        _ => Vec::new(),
    };
    let mut memory = st(ess).cooldowns.remove(&player).unwrap_or_default();
    let mut found = false;
    for list in [&mut saved, &mut memory] {
        let mut keep = Vec::new();
        for (pattern, until) in list.drain(..) {
            if until <= now {
                continue;
            }
            if cooldown_matches(&pattern, &full) {
                let left = format_date_diff(ess, now, until);
                ess.send_tl(host, Sender::Player(player), "commandCooldown", &[Arg::Plain(left)]);
                found = true;
            }
            keep.push((pattern, until));
        }
        *list = keep;
    }
    if !found {
        if let Some((pattern, ms)) = entries.into_iter().find(|(p, _)| cooldown_matches(p, &full)) {
            if ess.config_bool("command-cooldown-persistence", true) {
                saved.push((pattern, now + ms));
            } else {
                memory.push((pattern, now + ms));
            }
        }
    }
    let user = ess.users.user(player);
    let had = user.get("timestamps.command-cooldowns").is_some();
    if !saved.is_empty() || had {
        let value = if saved.is_empty() { Value::Null } else { Value::Array(saved.iter().map(|(p, v)| json!({"pattern": p, "value": v})).collect()) };
        user.set("timestamps.command-cooldowns", value);
    }
    if !memory.is_empty() {
        st(ess).cooldowns.insert(player, memory);
    }
    if found { Allow::No } else { Allow::Yes }
}

fn jailed_without(ess: &mut Ess, host: &mut dyn Host, player: Uuid, node: &str) -> Allow {
    if user_bool(ess, player, "jailed") && !has_node(ess, host, player, node) { Allow::No } else { Allow::Yes }
}

fn on_block_break(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _w: &str, _p: [i32; 3], _s: &str) -> Allow {
    jailed_without(ess, host, player, "essentials.jail.allow-break")
}

fn on_block_place(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _w: &str, _p: [i32; 3], _s: &str) -> Allow {
    jailed_without(ess, host, player, "essentials.jail.allow-place")
}

fn on_interact(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _b: Option<(&str, [i32; 3], &str)>, _i: Option<&Item>) -> Allow {
    jailed_without(ess, host, player, "essentials.jail.allow-interact")
}

fn on_respawn(ess: &mut Ess, _host: &mut dyn Host, player: Uuid) -> Option<Loc> {
    if !user_bool(ess, player, "jailed") {
        return None;
    }
    let jail = user_str(ess, player, "jail")?;
    jails(ess).get(&jail.to_lowercase()).cloned()
}

fn invsee_target(tag: &str) -> Option<(Uuid, bool)> {
    if let Some(u) = tag.strip_prefix("invsee-equip:") {
        return parse_uuid(u).map(|u| (u, true));
    }
    tag.strip_prefix("invsee:").and_then(parse_uuid).map(|u| (u, false))
}

/// Whether the viewer may change the owner's inventory through /invsee.
fn invsee_may_modify(ess: &mut Ess, host: &mut dyn Host, viewer: Uuid, owner: Uuid) -> bool {
    has_node(ess, host, viewer, "essentials.invsee.modify") && !has_node(ess, host, owner, "essentials.invsee.preventmodify") && is_online(host, owner)
}

fn on_container_click(ess: &mut Ess, host: &mut dyn Host, viewer: Uuid, tag: &str, _slot: usize) -> Allow {
    match invsee_target(tag) {
        Some((_, true)) => Allow::No,
        Some((owner, false)) if !invsee_may_modify(ess, host, viewer, owner) => Allow::No,
        _ => Allow::Yes,
    }
}

fn on_container_close(ess: &mut Ess, host: &mut dyn Host, viewer: Uuid, tag: &str, items: &[Option<Item>]) {
    let Some((owner, equip)) = invsee_target(tag) else { return };
    st(ess).invsee.remove(&viewer);
    if equip || !invsee_may_modify(ess, host, viewer, owner) {
        return;
    }
    let current = host.inventory(owner);
    for (slot, now) in current.iter().enumerate().take(41) {
        let new = items.get(slot).cloned().flatten();
        if *now != new {
            host.set_slot(owner, slot, new);
        }
    }
}

// ===== Bans ======================================================================================

/// The ban entry for a name or IP, dropping it when it has expired.
fn active_ban(host: &mut dyn Host, ip: bool, target: &str) -> Option<Ban> {
    let now = host.now_millis();
    let ban = host.bans(ip).into_iter().find(|b| b.target.eq_ignore_ascii_case(target))?;
    if ban.expires.is_some_and(|e| e <= now) {
        host.unban(ip, &ban.target);
        return None;
    }
    Some(ban)
}

fn default_ban_reason(ess: &Ess) -> String {
    plain(&ess.render(&ess.tl("defaultBanReason", &[])))
}

fn cmd_ban(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let target = find(ctx, &args[0], true, true).ok();
    let name = match target {
        Some(u) => name_of(ctx.ess, ctx.host, u),
        None => args[0].clone(),
    };
    let online = target.is_some_and(|u| is_online(ctx.host, u));
    if !online {
        if ctx.is_player() && !ctx.has("essentials.ban.offline") {
            return Err(CmdError::Tl("banExemptOffline", vec![]));
        }
    } else if ctx.is_player() && target.is_some_and(|u| has_node(ctx.ess, ctx.host, u, "essentials.ban.exempt")) {
        return Err(CmdError::Tl("banExempt", vec![]));
    }
    let sender = sender_display(ctx);
    let reason = if args.len() > 1 { reason_text(&final_arg(args, 1)) } else { default_ban_reason(ctx.ess) };
    ctx.host.ban(false, &Ban { target: name.clone(), reason: reason.clone(), source: sender.clone(), expires: None });
    let display = ctx.ess.tl("banFormat", &[Arg::Plain(reason.clone()), Arg::Plain(sender.clone())]);
    if let (Some(u), true) = (target, online) {
        kick_mini(ctx.ess, ctx.host, u, &display);
    }
    log_tl(ctx.ess, ctx.host, "playerBanned", &[Arg::Plain(sender.clone()), Arg::Plain(name.clone()), Arg::Parsed(display)]);
    if target.is_none() {
        ctx.tl("userUnknown", &[Arg::Plain(name.clone())]);
    }
    notify(ctx.ess, ctx.host, "essentials.ban.notify", "playerBanned", &[Arg::Plain(sender), Arg::Plain(name), Arg::Plain(reason)]);
    Ok(())
}

fn cmd_tempban(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let target = find(ctx, &args[0], true, true)?;
    let online = is_online(ctx.host, target);
    if !online && ctx.is_player() && !ctx.has("essentials.tempban.offline") {
        ctx.tl("tempbanExemptOffline", &[]);
        return Ok(());
    } else if ctx.is_player() && has_node(ctx.ess, ctx.host, target, "essentials.tempban.exempt") {
        ctx.tl("tempbanExempt", &[]);
        return Ok(());
    }
    let time = final_arg(args, 1);
    let now = ctx.host.now_millis();
    let until = parse_date_diff(&time, true, false, now).ok_or(CmdError::Tl("illegalDate", vec![]))?;
    let mut reason = replace_format(&remove_time_pattern(&time));
    let max = ctx.ess.config_i64("max-tempban-time", -1) * 1000;
    if max > 0 && until - now > max && ctx.is_player() && !ctx.has("essentials.tempban.unlimited") {
        ctx.tl("oversizedTempban", &[]);
        return Ok(());
    }
    if reason.chars().count() < 2 {
        reason = default_ban_reason(ctx.ess);
    }
    let sender = sender_display(ctx);
    let name = name_of(ctx.ess, ctx.host, target);
    ctx.host.ban(false, &Ban { target: name.clone(), reason: reason.clone(), source: sender.clone(), expires: Some(until) });
    let expiry = format_date_diff(ctx.ess, now, until);
    if online {
        let display = ctx.ess.tl("tempBanned", &[Arg::Plain(expiry.clone()), Arg::Plain(sender.clone()), Arg::Plain(reason.clone())]);
        kick_mini(ctx.ess, ctx.host, target, &display);
    }
    let a = [Arg::Plain(sender), Arg::Plain(name), Arg::Plain(expiry), Arg::Plain(reason)];
    log_tl(ctx.ess, ctx.host, "playerTempBanned", &a);
    notify(ctx.ess, ctx.host, "essentials.ban.notify", "playerTempBanned", &a);
    Ok(())
}

/// The IP an argument names: an address, or a known player's last login address.
fn ip_of(ctx: &mut Ctx, arg: &str) -> Result<String, CmdError> {
    let ip = if valid_ip(arg) {
        arg.to_string()
    } else {
        match find(ctx, arg, true, true) {
            Ok(u) => user_str(ctx.ess, u, "ip-address").unwrap_or_default(),
            Err(_) => arg.to_string(),
        }
    };
    if ip.is_empty() { Err(CmdError::PlayerNotFound) } else { Ok(ip) }
}

fn kick_address(ess: &mut Ess, host: &mut dyn Host, ip: &str, mini: &str) {
    for p in host.online() {
        if host.address(p).is_some_and(|a| a.eq_ignore_ascii_case(ip)) {
            kick_mini(ess, host, p, mini);
        }
    }
}

fn cmd_banip(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let sender = sender_display(ctx);
    let ip = ip_of(ctx, &args[0])?;
    let reason = if args.len() > 1 { reason_text(&final_arg(args, 1)) } else { default_ban_reason(ctx.ess) };
    let display = ctx.ess.tl("banFormat", &[Arg::Plain(reason.clone()), Arg::Plain(sender.clone())]);
    ctx.host.ban(true, &Ban { target: ip.clone(), reason: reason.clone(), source: sender.clone(), expires: None });
    let a = [Arg::Plain(sender), Arg::Plain(ip.clone()), Arg::Plain(reason)];
    log_tl(ctx.ess, ctx.host, "playerBanIpAddress", &a);
    kick_address(ctx.ess, ctx.host, &ip, &display);
    notify(ctx.ess, ctx.host, "essentials.banip.notify", "playerBanIpAddress", &a);
    Ok(())
}

fn cmd_tempbanip(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let sender = sender_display(ctx);
    let ip = ip_of(ctx, &args[0])?;
    let time = final_arg(args, 1);
    let now = ctx.host.now_millis();
    let until = parse_date_diff(&time, true, false, now).ok_or(CmdError::Tl("illegalDate", vec![]))?;
    let mut reason = replace_format(&remove_time_pattern(&time));
    let max = ctx.ess.config_i64("max-tempban-time", -1) * 1000;
    if max > 0 && until - now > max && ctx.is_player() && !ctx.has("essentials.tempban.unlimited") {
        ctx.tl("oversizedTempban", &[]);
        return Ok(());
    }
    if reason.chars().count() < 2 {
        reason = default_ban_reason(ctx.ess);
    }
    ctx.host.ban(true, &Ban { target: ip.clone(), reason: reason.clone(), source: sender.clone(), expires: Some(until) });
    let display = ctx.ess.tl("banFormat", &[Arg::Plain(reason.clone()), Arg::Plain(sender.clone())]);
    kick_address(ctx.ess, ctx.host, &ip, &display);
    // EssentialsX passes (sender, ip, reason, expiry, reason) to a message that reads {2} as the time.
    let expiry = format_date_diff(ctx.ess, now, until);
    let a = [Arg::Plain(sender), Arg::Plain(ip), Arg::Plain(reason.clone()), Arg::Plain(expiry), Arg::Plain(reason)];
    log_tl(ctx.ess, ctx.host, "playerTempBanIpAddress", &a);
    notify(ctx.ess, ctx.host, "essentials.banip.notify", "playerTempBanIpAddress", &a);
    Ok(())
}

fn cmd_unban(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let name = match find(ctx, &args[0], true, true) {
        Ok(u) => {
            let name = name_of(ctx.ess, ctx.host, u);
            ctx.host.unban(false, &name);
            name
        }
        Err(_) => {
            let banned = ctx.host.bans(false).into_iter().find(|b| b.target.eq_ignore_ascii_case(&args[0]));
            let Some(ban) = banned else { return Err(CmdError::Tl("playerNeverOnServer", vec![Arg::Plain(args[0].clone())])) };
            ctx.host.unban(false, &ban.target);
            ban.target
        }
    };
    let sender = sender_display(ctx);
    let a = [Arg::Plain(sender), Arg::Plain(name)];
    log_tl(ctx.ess, ctx.host, "playerUnbanned", &a);
    notify(ctx.ess, ctx.host, "essentials.ban.notify", "playerUnbanned", &a);
    Ok(())
}

fn cmd_unbanip(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let ip = ip_of(ctx, &args[0])?;
    ctx.host.unban(true, &ip);
    let sender = sender_display(ctx);
    let a = [Arg::Plain(sender), Arg::Plain(ip)];
    log_tl(ctx.ess, ctx.host, "playerUnbanIpAddress", &a);
    notify(ctx.ess, ctx.host, "essentials.banip.notify", "playerUnbanIpAddress", &a);
    Ok(())
}

// ===== Kick, kill, sudo ===========================================================================

fn cmd_kick(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let target = find(ctx, &args[0], true, false)?;
    if ctx.is_player() {
        if hidden_from(ctx.ess, ctx.host, ctx.sender, target) && !can_interact_vanished(ctx.ess, ctx.host, ctx.sender) {
            return Err(CmdError::PlayerNotFound);
        }
        if has_node(ctx.ess, ctx.host, target, "essentials.kick.exempt") {
            return Err(CmdError::Tl("kickExempt", vec![]));
        }
    }
    let raw = if args.len() > 1 { final_arg(args, 1) } else { plain(&ctx.ess.render(&ctx.ess.tl("kickDefault", &[]))) };
    let reason = reason_text(&raw);
    kick_mini(ctx.ess, ctx.host, target, &legacy_mini(&reason));
    let sender = sender_display(ctx);
    let name = name_of(ctx.ess, ctx.host, target);
    let a = [Arg::Plain(sender), Arg::Plain(name), Arg::Plain(reason)];
    log_tl(ctx.ess, ctx.host, "playerKicked", &a);
    notify(ctx.ess, ctx.host, "essentials.kick.notify", "playerKicked", &a);
    Ok(())
}

fn cmd_kickall(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let raw = if args.is_empty() { plain(&ctx.ess.render(&ctx.ess.tl("kickDefault", &[]))) } else { final_arg(args, 0) };
    let reason = legacy_mini(&reason_text(&raw));
    for p in ctx.host.online() {
        if Sender::Player(p) != ctx.sender && !has_node(ctx.ess, ctx.host, p, "essentials.kickall.exempt") {
            kick_mini(ctx.ess, ctx.host, p, &reason);
        }
    }
    ctx.tl("kickedAll", &[]);
    Ok(())
}

fn cmd_kill(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let force = !ctx.is_player() || ctx.has("essentials.kill.force");
    for target in loop_players(ctx, true, true, &args[0])? {
        let display = display_name(ctx.ess, ctx.host, target);
        if ctx.is_player() && has_node(ctx.ess, ctx.host, target, "essentials.kill.exempt") && !force {
            return Err(CmdError::Tl("killExempt", vec![Arg::Plain(display)]));
        }
        let cause = if ctx.sender == Sender::Player(target) { "suicide" } else { "custom" };
        let allowed = ctx.ess.on_damage(ctx.host, target, cause, f64::MAX);
        if allowed == Allow::No && ctx.is_player() && !force {
            continue;
        }
        ctx.host.set_health(target, 0.0);
        ctx.tl("kill", &[Arg::Plain(display)]);
    }
    Ok(())
}

/// The player "types" a line: Essentials runs its own commands, vanilla the rest.
pub fn run_as(ess: &mut Ess, host: &mut dyn Host, player: Uuid, line: &str) {
    if ess.claims(line) {
        ess.dispatch(host, Sender::Player(player), line);
    } else {
        host.player_command(player, line, false);
    }
}

/// player.chat(message): chat listeners first, then a vanilla-looking line.
pub fn chat_as(ess: &mut Ess, host: &mut dyn Host, player: Uuid, message: &str) {
    match ess.on_chat(host, player, message) {
        Chat::Cancel => {}
        Chat::Replace(lines) => {
            for (to, spans) in lines {
                match to {
                    None => host.broadcast(&spans),
                    Some(list) => {
                        for p in list {
                            host.send(Sender::Player(p), &spans);
                        }
                    }
                }
            }
        }
        Chat::Pass => {
            let name = display_name(ess, host, player);
            let spans = ess.render(&format!("\\<{}> {}", legacy_mini(&name), escape_tags(message)));
            host.chat_as(player, &spans);
        }
    }
}

fn cmd_sudo(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let line = final_arg(args, 1);
    let multiple = !ctx.is_player() || ctx.has("essentials.sudo.multiple");
    ctx.tl("sudoRun", &[Arg::Plain(args[0].clone()), Arg::Plain(line.clone()), Arg::Plain(String::new())]);
    let me = sender_name(ctx);
    for target in loop_players(ctx, false, multiple, &args[0])? {
        let name = name_of(ctx.ess, ctx.host, target);
        if name == me {
            continue;
        }
        if ctx.is_player() && has_node(ctx.ess, ctx.host, target, "essentials.sudo.exempt") {
            ctx.tl("sudoExempt", &[Arg::Plain(name)]);
            continue;
        }
        if line.to_lowercase().starts_with("c:") {
            let message: String = line.chars().skip(2).collect();
            chat_as(ctx.ess, ctx.host, target, &message);
            continue;
        }
        if !line.is_empty() {
            let line = line.clone();
            ctx.ess.scheduler.later(0, Box::new(move |ess: &mut Ess, host: &mut dyn Host| run_as(ess, host, target, &line)));
        }
    }
    Ok(())
}

// ===== Vanish, list, near ========================================================================

/// Online players who may see vanished players (essentials.vanish.see).
fn seers(ess: &mut Ess, host: &mut dyn Host) -> Vec<Uuid> {
    let mut out = Vec::new();
    for p in host.online() {
        if has_node(ess, host, p, "essentials.vanish.see") {
            out.push(p);
        }
    }
    out
}

/// User.setVanished.
pub fn set_vanished(ess: &mut Ess, host: &mut dyn Host, player: Uuid, on: bool) {
    let seers = seers(ess, host);
    host.set_hidden(player, on, &seers);
    ess.users.user(player).set("vanished", json!(on));
    let now = host.now_millis();
    if on {
        st(ess).vanished_at.insert(player, now);
    } else {
        st(ess).vanished_at.remove(&player);
    }
    if has_node(ess, host, player, "essentials.vanish.effect") {
        let name = name_of(ess, host, player);
        if on {
            host.console_command(&format!("effect give {name} minecraft:invisibility infinite 1 false"));
        } else {
            host.console_command(&format!("effect clear {name} minecraft:invisibility"));
        }
    }
}

fn match_toggle(arg: &str) -> Option<bool> {
    let a = arg.to_lowercase();
    if a == "on" || a.starts_with("ena") || a == "1" {
        Some(true)
    } else if a == "off" || a.starts_with("dis") || a == "0" {
        Some(false)
    } else {
        None
    }
}

fn toggle_vanish(ctx: &mut Ctx, player: Uuid, enabled: Option<bool>) {
    let on = enabled.unwrap_or_else(|| !ctx.ess.is_vanished(player));
    set_vanished(ctx.ess, ctx.host, player, on);
    let display = display_name(ctx.ess, ctx.host, player);
    let state = Arg::Parsed(ctx.ess.tl(if on { "enabled" } else { "disabled" }, &[]));
    ctx.tl_to(Sender::Player(player), "vanish", &[Arg::Plain(display.clone()), state.clone()]);
    if on {
        ctx.tl_to(Sender::Player(player), "vanished", &[]);
    }
    if ctx.sender != Sender::Player(player) {
        ctx.tl("vanish", &[Arg::Plain(display), state]);
    }
}

fn toggle_vanish_others(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() || args[0].trim().chars().count() < 2 {
        return Err(CmdError::PlayerNotFound);
    }
    let skip_hidden = ctx.is_player() && !can_interact_vanished(ctx.ess, ctx.host, ctx.sender);
    let mut found = false;
    for p in match_players(ctx.host, &args[0]) {
        if skip_hidden && hidden_from(ctx.ess, ctx.host, ctx.sender, p) {
            continue;
        }
        found = true;
        toggle_vanish(ctx, p, args.get(1).and_then(|a| match_toggle(a)));
    }
    if found { Ok(()) } else { Err(CmdError::PlayerNotFound) }
}

fn cmd_vanish(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else { return toggle_vanish_others(ctx, args) };
    let others = ctx.has("essentials.vanish.others");
    if args.len() == 1 {
        let toggle = match_toggle(&args[0]);
        if toggle.is_none() && others {
            return toggle_vanish_others(ctx, args);
        }
        toggle_vanish(ctx, me, toggle);
    } else if args.len() == 2 && others {
        return toggle_vanish_others(ctx, args);
    } else {
        toggle_vanish(ctx, me, None);
    }
    Ok(())
}

/// PlayerList.listUsers.
fn list_users(ess: &mut Ess, host: &mut dyn Host, users: &[Uuid], separator: &str) -> String {
    let mut named: Vec<(String, Uuid)> = users.iter().map(|u| (display_name(ess, host, *u), *u)).collect();
    named.sort_by_key(|(d, _)| strip_legacy(d, false).to_lowercase());
    let mut out = Vec::new();
    for (display, u) in named {
        let mut s = String::new();
        if user_bool(ess, u, "afk") {
            s.push_str(&ess.tl("listAfkTag", &[]));
        }
        if ess.is_vanished(u) {
            s.push_str(&ess.tl("listHiddenTag", &[]));
        }
        s.push_str(&legacy_mini(&display));
        let name = name_of(ess, host, u);
        let nick = user_str(ess, u, "nickname").map(|n| strip_legacy(&n, true));
        if ess.config_bool("real-names-on-list", false) && nick.is_some_and(|n| n != name) {
            s.push(' ');
            s.push_str(&ess.tl("listRealName", &[Arg::Plain(name)]));
        }
        s.push_str("<white>");
        out.push(s);
    }
    out.join(separator)
}

/// The `list` section of config.yml: (group, value) in file order.
fn list_config(ess: &Ess) -> Vec<(String, String)> {
    match ess.config("list") {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()).trim().to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn merged_list(ess: &Ess, lists: &mut BTreeMap<String, Vec<Uuid>>, group: &str) -> Vec<Uuid> {
    let mut users = Vec::new();
    for (config_group, value) in list_config(ess) {
        if config_group.eq_ignore_ascii_case(group) {
            for g in value.split(' ').map(|g| g.trim().to_lowercase()).filter(|g| !g.is_empty()) {
                if let Some(u) = lists.remove(&g) {
                    users.extend(u);
                }
            }
        }
    }
    users
}

fn output_format(ess: &Ess, group: &str, message: &str) -> String {
    format!("{}{}", ess.tl("listGroupTag", &[Arg::Plain(replace_format(group))]), message)
}

fn grouped_list(ctx: &mut Ctx, mut lists: BTreeMap<String, Vec<Uuid>>) -> Vec<String> {
    let mut out = Vec::new();
    let mut asterisk = Vec::new();
    for (original, value) in list_config(ctx.ess) {
        let group = original.to_lowercase();
        if value == "*" {
            asterisk.push(original);
            continue;
        }
        if value.eq_ignore_ascii_case("hidden") {
            lists.remove(&group);
            continue;
        }
        if let Ok(limit) = value.parse::<usize>() {
            if let Some(matched) = lists.remove(&group).filter(|m| !m.is_empty()) {
                let text = if matched.len() > limit {
                    ctx.ess.tl("groupNumber", &[Arg::from(matched.len()), Arg::Plain(ctx.label.clone()), Arg::Plain(strip_legacy(&group, false))])
                } else {
                    list_users(ctx.ess, ctx.host, &matched, ", ")
                };
                out.push(output_format(ctx.ess, &original, &text));
                continue;
            }
        }
        let users = merged_list(ctx.ess, &mut lists, &group);
        if users.is_empty() {
            continue;
        }
        let text = list_users(ctx.ess, ctx.host, &users, ", ");
        out.push(output_format(ctx.ess, &original, &text));
    }
    let mut groups: Vec<String> = lists.keys().cloned().collect();
    groups.sort_by_key(|g| g.to_lowercase());
    if !asterisk.is_empty() {
        let everyone: Vec<Uuid> = groups.iter().flat_map(|g| lists[g].clone()).collect();
        for key in &asterisk {
            lists.insert(key.clone(), everyone.clone());
        }
        groups = asterisk.clone();
    }
    for g in groups {
        let users = lists.get(&g).cloned().unwrap_or_default();
        if users.is_empty() {
            continue;
        }
        let name = if asterisk.is_empty() { ctx.ess.group(ctx.host, users[0]) } else { g };
        let text = list_users(ctx.ess, ctx.host, &users, ", ");
        out.push(output_format(ctx.ess, &name, &text));
    }
    out
}

fn cmd_list(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let viewer = ctx.sender;
    let show_hidden = !ctx.is_player() || ctx.has("essentials.list.hidden") || can_interact_vanished(ctx.ess, ctx.host, viewer);
    let online = ctx.host.online();
    let (mut player_hidden, mut hidden_count) = (0, 0);
    let mut lists: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();
    for &p in &online {
        let vanished = ctx.ess.is_vanished(p);
        let from = ctx.is_player() && hidden_from(ctx.ess, ctx.host, viewer, p);
        if vanished || from {
            player_hidden += 1;
            if show_hidden || (ctx.is_player() && !from) {
                hidden_count += 1;
            }
        }
        if (!ctx.is_player() && !show_hidden && vanished) || (ctx.is_player() && !show_hidden && from) {
            continue;
        }
        let group = strip_legacy(&ctx.ess.group(ctx.host, p), true).to_lowercase();
        lists.entry(group).or_default().push(p);
    }
    let max = ctx.host.max_players();
    if hidden_count > 0 {
        ctx.tl("listAmountHidden", &[Arg::from(online.len() - player_hidden), Arg::from(hidden_count), Arg::from(max)]);
    } else {
        ctx.tl("listAmount", &[Arg::from(online.len() - player_hidden), Arg::from(max)]);
    }
    if let Some(group) = args.first() {
        let group = group.to_lowercase();
        let mut users = merged_list(ctx.ess, &mut lists, &group);
        users.extend(lists.get(&group).cloned().unwrap_or_default());
        if users.is_empty() {
            return Err(CmdError::Tl("groupDoesNotExist", vec![]));
        }
        let mut title: Vec<char> = group.chars().collect();
        title[0] = title[0].to_uppercase().next().unwrap_or(title[0]);
        let text = list_users(ctx.ess, ctx.host, &users, ", ");
        let line = output_format(ctx.ess, &title.into_iter().collect::<String>(), &text);
        ctx.send_mini(&line);
        return Ok(());
    }
    for line in grouped_list(ctx, lists) {
        ctx.send_mini(&line);
    }
    Ok(())
}

fn near_list(ctx: &mut Ctx, center: Uuid, radius: i64) -> String {
    let Some(at) = ctx.host.location(center) else { return ctx.ess.tl("none", &[]) };
    let show_hidden = can_interact_vanished(ctx.ess, ctx.host, Sender::Player(center));
    let mut near: Vec<(f64, Uuid)> = Vec::new();
    for p in ctx.host.online() {
        if p == center || has_node(ctx.ess, ctx.host, p, "essentials.near.exclude") {
            continue;
        }
        if ctx.ess.is_vanished(p) && !show_hidden && hidden_from(ctx.ess, ctx.host, Sender::Player(center), p) {
            continue;
        }
        let Some(l) = ctx.host.location(p) else { continue };
        if l.world != at.world {
            continue;
        }
        let d2 = (l.x - at.x).powi(2) + (l.y - at.y).powi(2) + (l.z - at.z).powi(2);
        if (d2 as i64) < radius.saturating_mul(radius) {
            near.push((d2, p));
        }
    }
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    let parts: Vec<String> = near
        .into_iter()
        .map(|(d2, p)| {
            let display = display_name(ctx.ess, ctx.host, p);
            ctx.ess.tl("nearbyPlayersList", &[Arg::Plain(display), Arg::from(d2.sqrt() as i64)])
        })
        .collect();
    if parts.is_empty() { ctx.ess.tl("none", &[]) } else { parts.join(", ") }
}

fn cmd_near(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else {
        if args.is_empty() {
            return Err(CmdError::Usage(None));
        }
        let other = find(ctx, &args[0], true, false)?;
        let radius = args.get(1).and_then(|a| a.parse::<i64>().ok()).unwrap_or(200);
        let list = near_list(ctx, other, radius);
        ctx.tl("nearbyPlayers", &[Arg::Parsed(list)]);
        return Ok(());
    };
    let mut max = ctx.ess.config_i64("near-radius", 200);
    if max == 0 {
        max = 200;
    }
    let mut radius = max;
    let mut other = None;
    if let Some(first) = args.first() {
        match first.parse::<i64>() {
            Ok(r) => radius = r,
            Err(_) => other = find(ctx, first, false, false).ok(),
        }
        if other.is_some() {
            if let Some(r) = args.get(1).and_then(|a| a.parse::<i64>().ok()) {
                radius = r;
            }
        }
    }
    radius = radius.abs();
    if radius > max && !ctx.has("essentials.near.maxexempt") {
        ctx.tl("radiusTooBig", &[Arg::from(max)]);
        radius = max;
    }
    let center = match other {
        Some(o) if ctx.has("essentials.near.others") => o,
        _ => me,
    };
    let list = near_list(ctx, center, radius);
    ctx.tl("nearbyPlayers", &[Arg::Parsed(list)]);
    Ok(())
}

// ===== Positions and server info =================================================================

fn output_position(ctx: &mut Ctx, at: &Loc, from: Option<&Loc>) {
    let [x, y, z] = at.block();
    ctx.tl("currentWorld", &[Arg::Plain(world_name(&at.world))]);
    ctx.tl("posX", &[Arg::from(x)]);
    ctx.tl("posY", &[Arg::from(y)]);
    ctx.tl("posZ", &[Arg::from(z)]);
    ctx.tl("posYaw", &[Arg::Plain(format_number(f64::from((at.yaw + 360.0) % 360.0), 3))]);
    ctx.tl("posPitch", &[Arg::Plain(format_number(f64::from(at.pitch), 3))]);
    if let Some(from) = from.filter(|f| f.world == at.world) {
        ctx.tl("distance", &[Arg::Plain(format_number(at.distance(from), 3))]);
    }
}

fn cmd_getpos(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    match ctx.sender {
        Sender::Player(me) => {
            let mine = ctx.host.location(me).ok_or(CmdError::Quiet)?;
            if !args.is_empty() && ctx.has("essentials.getpos.others") {
                let other = find(ctx, &args[0], false, false)?;
                let at = ctx.host.location(other).ok_or(CmdError::PlayerNotFound)?;
                output_position(ctx, &at, Some(&mine));
            } else {
                output_position(ctx, &mine, None);
            }
        }
        Sender::Console => {
            if args.is_empty() {
                return Err(CmdError::Usage(None));
            }
            let other = find(ctx, &args[0], true, false)?;
            let at = ctx.host.location(other).ok_or(CmdError::PlayerNotFound)?;
            output_position(ctx, &at, None);
        }
    }
    Ok(())
}

fn cmd_compass(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let at = ctx.host.location(me).ok_or(CmdError::Quiet)?;
    let bearing = ((at.yaw + 180.0 + 360.0) as i64).rem_euclid(360);
    let key = match bearing {
        b if b < 23 => "north",
        b if b < 68 => "northEast",
        b if b < 113 => "east",
        b if b < 158 => "southEast",
        b if b < 203 => "south",
        b if b < 248 => "southWest",
        b if b < 293 => "west",
        b if b < 338 => "northWest",
        _ => "north",
    };
    let dir = ctx.ess.tl(key, &[]);
    ctx.tl("compassBearing", &[Arg::Parsed(dir), Arg::from(bearing)]);
    Ok(())
}

/// World.getSeaLevel() for vanilla dimensions.
fn sea_level(world: &str) -> i32 {
    match world {
        "minecraft:the_nether" => 32,
        "minecraft:the_end" => 0,
        _ => 63,
    }
}

fn cmd_depth(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let at = ctx.host.location(me).ok_or(CmdError::Quiet)?;
    let depth = at.block()[1] - sea_level(&at.world);
    match depth {
        d if d > 0 => ctx.tl("depthAboveSea", &[Arg::from(d)]),
        d if d < 0 => ctx.tl("depthBelowSea", &[Arg::from(-d)]),
        _ => ctx.tl("depth", &[]),
    }
    Ok(())
}

fn cmd_gc(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let stats = ctx.host.stats();
    let now = ctx.host.now_millis();
    let uptime = format_date_diff(ctx.ess, now, now - stats.uptime_ms);
    ctx.tl("uptime", &[Arg::Plain(uptime)]);
    let color = if stats.tps >= 18.0 {
        "§a"
    } else if stats.tps >= 15.0 {
        "§e"
    } else {
        "§c"
    };
    ctx.tl("tps", &[Arg::Plain(format!("{color}{}", format_number(stats.tps, 2)))]);
    ctx.tl("gcmax", &[Arg::from(stats.memory_max_mb)]);
    ctx.tl("gctotal", &[Arg::from(stats.memory_used_mb)]);
    ctx.tl("gcfree", &[Arg::from(stats.memory_max_mb.saturating_sub(stats.memory_used_mb))]);
    for (world, chunks, entities, tiles) in stats.worlds {
        let kind = match world.as_str() {
            "minecraft:the_nether" => "Nether",
            "minecraft:the_end" => "The End",
            _ => "World",
        };
        ctx.tl("gcWorld", &[Arg::Plain(kind.into()), Arg::Plain(world_name(&world)), Arg::from(chunks), Arg::from(entities), Arg::from(tiles)]);
    }
    Ok(())
}

fn cmd_playtime(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (display, ticks, key) = if !args.is_empty() && ctx.has("essentials.playtime.others") {
        match find(ctx, &args[0], false, false) {
            Ok(u) => (display_name(ctx.ess, ctx.host, u), ctx.host.statistic(u, PLAY_TIME), "playtimeOther"),
            Err(_) => {
                let u = find(ctx, &args[0], true, true)?;
                let mut ticks = ctx.host.statistic(u, PLAY_TIME);
                if is_online(ctx.host, u) && ctx.ess.is_vanished(u) {
                    let since = st(ctx.ess).vanished_at.get(&u).copied().unwrap_or(0);
                    ticks -= (ctx.host.now_millis() - since) / 50;
                }
                (name_of(ctx.ess, ctx.host, u), ticks, "playtimeOther")
            }
        }
    } else if let Sender::Player(me) = ctx.sender {
        (display_name(ctx.ess, ctx.host, me), ctx.host.statistic(me, PLAY_TIME), "playtime")
    } else {
        return Err(CmdError::Usage(None));
    };
    let since = ctx.host.now_millis() - ticks * 50;
    let diff = date_diff_from_now(ctx, since);
    ctx.tl(key, &[Arg::Plain(diff), Arg::Parsed(legacy_mini(&display))]);
    Ok(())
}

// ===== whois / seen ===============================================================================

/// Vanilla's experience level for a total amount of experience points.
fn level_for(total: i64) -> i64 {
    let mut level = 0;
    let mut left = total;
    loop {
        let need = if level <= 15 { 2 * level + 7 } else if level <= 30 { 5 * level - 38 } else { 9 * level - 158 };
        if left < need {
            return level;
        }
        left -= need;
        level += 1;
    }
}

/// NumberUtil.displayCurrency.
fn display_currency(ess: &Ess, value: f64) -> String {
    let floored = (value.abs() * 100.0).floor() / 100.0;
    let mut amount = crate::game::ess_text::group_digits(&format!("{floored:.2}"));
    if amount.ends_with(".00") {
        amount.truncate(amount.len() - 3);
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let symbol = ess.config_str("currency-symbol").unwrap_or_else(|| "$".into());
    if ess.config_bool("currency-symbol-suffix", false) {
        format!("{sign}{}", ess.tl("currency", &[Arg::Plain(amount), Arg::Plain(symbol)]))
    } else {
        format!("{sign}{}", ess.tl("currency", &[Arg::Plain(symbol), Arg::Plain(amount)]))
    }
}

fn money_of(ess: &mut Ess, player: Uuid) -> f64 {
    let start = ess.config_f64("starting-balance", 0.0);
    match ess.users.get(player).and_then(|u| u.get("money").cloned()) {
        Some(Value::String(s)) => s.parse().unwrap_or(start),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(start),
        _ => start,
    }
}

/// The "muted" line (whoisMuted / whoisMutedReason) when `always` or the player is muted.
fn muted_line(ctx: &mut Ctx, player: Uuid, always: bool) {
    let muted = user_bool(ctx.ess, player, "muted");
    if !muted && !always {
        return;
    }
    let until = user_i64(ctx.ess, player, "timestamps.mute");
    let value = if !muted {
        ctx.ess.tl("false", &[])
    } else if until > 0 {
        legacy_mini(&date_diff_from_now(ctx, until))
    } else {
        ctx.ess.tl("true", &[])
    };
    match user_str(ctx.ess, player, "mute-reason").filter(|r| !r.is_empty()) {
        Some(reason) => ctx.tl("whoisMutedReason", &[Arg::Parsed(value), Arg::Plain(reason)]),
        None => ctx.tl("whoisMuted", &[Arg::Parsed(value)]),
    }
}

fn cmd_whois(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let u = find(ctx, &args[0], false, false)?;
    let name = name_of(ctx.ess, ctx.host, u);
    ctx.tl("whoisTop", &[Arg::Plain(name.clone())]);
    let display = display_name(ctx.ess, ctx.host, u);
    ctx.tl("whoisNick", &[Arg::Plain(display)]);
    ctx.tl("whoisUuid", &[Arg::Plain(uuid_string(u))]);
    let (health, _) = ctx.host.health(u);
    ctx.tl("whoisHealth", &[Arg::Plain(format_number(health, 3))]);
    let (food, saturation) = ctx.host.food(u);
    ctx.tl("whoisHunger", &[Arg::from(food), Arg::Plain(format_number(f64::from(saturation), 3))]);
    let exp = ctx.host.exp(u);
    ctx.tl("whoisExp", &[Arg::from(exp), Arg::from(level_for(exp))]);
    let at = ctx.host.location(u).ok_or(CmdError::PlayerNotFound)?;
    let [x, y, z] = at.block();
    ctx.tl("whoisLocation", &[Arg::Plain(world_name(&at.world)), Arg::from(x), Arg::from(y), Arg::from(z)]);
    let played = ctx.host.statistic(u, PLAY_TIME);
    let since = ctx.host.now_millis() - played * 50;
    let diff = date_diff_from_now(ctx, since);
    ctx.tl("whoisPlaytime", &[Arg::Plain(diff)]);
    if !ctx.ess.config_bool("disable-eco", false) {
        let money = money_of(ctx.ess, u);
        let text = display_currency(ctx.ess, money);
        ctx.tl("whoisMoney", &[Arg::Parsed(text)]);
    }
    if !ctx.is_player() || ctx.has("essentials.whois.ip") {
        let ip = ctx.host.address(u).unwrap_or_default();
        ctx.tl("whoisIPAddress", &[Arg::Plain(format!("/{ip}"))]);
    }
    let mode = ctx.host.game_mode(u);
    let mode = ctx.ess.tl(mode.name(), &[]);
    ctx.tl("whoisGamemode", &[Arg::Parsed(mode)]);
    let god = user_bool(ctx.ess, u, "godmode");
    let god = tl_bool(ctx.ess, god);
    ctx.tl("whoisGod", &[god]);
    let op = tl_bool(ctx.ess, ctx.host.op_level(u) > 0);
    ctx.tl("whoisOp", &[op]);
    let flying = ctx.host.flying(u);
    let allow = tl_bool(ctx.ess, ctx.host.allow_flight(u));
    let state = ctx.ess.tl(if flying { "flying" } else { "notFlying" }, &[]);
    ctx.tl("whoisFly", &[allow, Arg::Parsed(state)]);
    let (fly, walk) = ctx.host.speeds(u);
    ctx.tl("whoisSpeed", &[Arg::Plain(format_number(f64::from(if flying { fly } else { walk }), 3))]);
    let white = tl_bool(ctx.ess, ctx.host.whitelisted(u));
    ctx.tl("whoisWhitelist", &[white]);
    let afk = user_bool(ctx.ess, u, "afk");
    let afk_since = user_i64(ctx.ess, u, "timestamps.afk-since");
    if afk && afk_since > 0 {
        let diff = date_diff_from_now(ctx, afk_since);
        let t = tl_bool(ctx.ess, true);
        ctx.tl("whoisAFKSince", &[t, Arg::Plain(diff)]);
    } else {
        let t = tl_bool(ctx.ess, afk);
        ctx.tl("whoisAFK", &[t]);
    }
    let jail = if user_bool(ctx.ess, u, "jailed") {
        if user_i64(ctx.ess, u, "timestamps.jail") > 0 { legacy_mini(&formatted_jail_time(ctx.ess, ctx.host, u)) } else { ctx.ess.tl("true", &[]) }
    } else {
        ctx.ess.tl("false", &[])
    };
    ctx.tl("whoisJail", &[Arg::Parsed(jail)]);
    muted_line(ctx, u, true);
    Ok(())
}

fn seen_ip(ctx: &mut Ctx, ip: &str, display: &str) {
    ctx.tl("runningPlayerMatch", &[Arg::Parsed(legacy_mini(display))]);
    let mut matches = Vec::new();
    for u in ctx.ess.users.all() {
        if user_str(ctx.ess, u, "ip-address").is_some_and(|a| !a.is_empty() && a.eq_ignore_ascii_case(ip)) {
            matches.push(name_of(ctx.ess, ctx.host, u));
        }
    }
    matches.sort_by_key(|n| n.to_lowercase());
    if matches.is_empty() {
        ctx.tl("noMatchingPlayers", &[]);
    } else {
        ctx.tl("matchingIPAddress", &[]);
        ctx.tl("matchingAccounts", &[Arg::Plain(matches.join(", "))]);
    }
}

fn past_names(ess: &mut Ess, player: Uuid) -> Vec<String> {
    match ess.users.get(player).and_then(|u| u.get("past-usernames").cloned()) {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

fn ban_lines(ctx: &mut Ctx, name: &str, show_reason: bool) {
    let Some(ban) = active_ban(ctx.host, false, name) else { return };
    let reason = if show_reason { Arg::Plain(ban.reason.clone()) } else { tl_bool(ctx.ess, true) };
    ctx.tl("whoisBanned", &[reason]);
    if let Some(e) = ban.expires {
        let now = ctx.host.now_millis();
        let when = if e > now { Arg::Plain(format_date_diff(ctx.ess, now, e)) } else { Arg::Parsed(ctx.ess.tl("now", &[])) };
        ctx.tl("whoisTempBanned", &[when]);
    }
}

fn cmd_seen(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let show_ban = ctx.has("essentials.seen.banreason");
    let show_ip = ctx.has("essentials.seen.ip");
    let show_location = ctx.has("essentials.seen.location");
    let show_whitelist = ctx.has("essentials.seen.whitelist");
    let search_accounts = ctx.label.contains("alts") && ctx.has("essentials.seen.alts");
    let known = match parse_uuid(&args[0]) {
        Some(u) => ctx.ess.users.exists(u).then_some(u),
        None => ctx.ess.users.names.get(&args[0].to_lowercase()).copied(),
    };
    let player = match known {
        Some(u) => u,
        None => {
            if !search_accounts {
                let ip_banned = active_ban(ctx.host, true, &args[0]).is_some();
                if ctx.has("essentials.seen.ipsearch") && valid_ip(&args[0]) {
                    if ip_banned {
                        ctx.tl("isIpBanned", &[Arg::Plain(args[0].clone())]);
                    }
                    seen_ip(ctx, &args[0], &args[0]);
                    return Ok(());
                } else if ip_banned {
                    ctx.tl("isIpBanned", &[Arg::Plain(args[0].clone())]);
                    return Ok(());
                } else if let Some(ban) = active_ban(ctx.host, false, &args[0]) {
                    let reason = if show_ban { Arg::Plain(ban.reason) } else { tl_bool(ctx.ess, true) };
                    ctx.tl("whoisBanned", &[reason]);
                    return Ok(());
                }
            }
            find(ctx, &args[0], false, false).map_err(|_| CmdError::Tl("playerNeverOnServer", vec![Arg::Plain(args[0].clone())]))?
        }
    };
    let name = name_of(ctx.ess, ctx.host, player);
    if search_accounts {
        let ip = user_str(ctx.ess, player, "ip-address").unwrap_or_default();
        let display = display_name(ctx.ess, ctx.host, player);
        seen_ip(ctx, &ip, &display);
        return Ok(());
    }
    let visible = !ctx.is_player() || !ctx.ess.is_vanished(player) || can_interact_vanished(ctx.ess, ctx.host, ctx.sender);
    if is_online(ctx.host, player) && visible {
        let display = display_name(ctx.ess, ctx.host, player);
        let login = user_i64(ctx.ess, player, "timestamps.login");
        let since = date_diff_from_now(ctx, login);
        ctx.tl("seenOnline", &[Arg::Plain(display), Arg::Plain(since)]);
        let history: Vec<String> = past_names(ctx.ess, player).into_iter().filter(|n| *n != name).collect();
        if !history.is_empty() {
            ctx.tl("seenAccounts", &[Arg::Plain(history.join(", "))]);
        }
        if ctx.has("essentials.seen.uuid") {
            ctx.tl("whoisUuid", &[Arg::Plain(uuid_string(player))]);
        }
        if ctx.has("essentials.seen.firstlogin") {
            if let Some(first) = ctx.host.first_played(player) {
                ctx.tl("whoisFirstLogin", &[Arg::Plain(format_date(first))]);
            }
        }
        if user_bool(ctx.ess, player, "afk") {
            let t = tl_bool(ctx.ess, true);
            ctx.tl("whoisAFK", &[t]);
        }
        if user_bool(ctx.ess, player, "jailed") {
            let value = if user_i64(ctx.ess, player, "timestamps.jail") > 0 { Arg::Plain(formatted_jail_time(ctx.ess, ctx.host, player)) } else { tl_bool(ctx.ess, true) };
            ctx.tl("whoisJail", &[value]);
        }
        muted_line(ctx, player, false);
        if show_ip {
            let ip = ctx.host.address(player).unwrap_or_default();
            ctx.tl("whoisIPAddress", &[Arg::Plain(format!("/{ip}"))]);
        }
        return Ok(());
    }
    let logout = user_i64(ctx.ess, player, "timestamps.logout");
    if logout > 0 {
        let since = date_diff_from_now(ctx, logout);
        ctx.tl("seenOffline", &[Arg::Plain(name.clone()), Arg::Plain(since)]);
        let history = past_names(ctx.ess, player);
        if history.len() > 1 {
            let others: Vec<String> = history.into_iter().filter(|n| *n != name).collect();
            ctx.tl("seenAccounts", &[Arg::Plain(others.join(", "))]);
        }
        if ctx.has("essentials.seen.uuid") {
            ctx.tl("whoisUuid", &[Arg::Plain(uuid_string(player))]);
        }
    } else {
        ctx.tl("userUnknown", &[Arg::Plain(name.clone())]);
    }
    if show_whitelist {
        let white = tl_bool(ctx.ess, ctx.host.whitelisted(player));
        ctx.tl("whoisWhitelist", &[white]);
    }
    ban_lines(ctx, &name, show_ban);
    muted_line(ctx, player, false);
    if show_ip {
        if let Some(ip) = user_str(ctx.ess, player, "ip-address").filter(|i| !i.is_empty()) {
            ctx.tl("whoisIPAddress", &[Arg::Plain(ip)]);
        }
    }
    if show_location {
        if let Some(loc) = ctx.ess.users.get(player).and_then(|u| u.get("logoutlocation").cloned()).and_then(|v| Loc::from_yaml(&v)) {
            let [x, y, z] = loc.block();
            ctx.tl("whoisLocation", &[Arg::Plain(world_name(&loc.world)), Arg::from(x), Arg::from(y), Arg::from(z)]);
        }
    }
    Ok(())
}

// ===== invsee =====================================================================================

fn cmd_invsee(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let target = find(ctx, &args[0], false, false)?;
    if target == me {
        ctx.tl("invseeNoSelf", &[]);
        return Err(CmdError::Quiet);
    }
    let inventory = ctx.host.inventory(target);
    let container = if args.len() > 1 && ctx.has("essentials.invsee.equip") {
        let mut items: Vec<Option<Item>> = vec![None; 9];
        for (i, slot) in (36..40).enumerate() {
            items[i] = inventory.get(slot).cloned().flatten();
        }
        items[4] = inventory.get(40).cloned().flatten();
        Container { rows: 1, title: ctx.ess.tl("equipped", &[]), items, tag: format!("invsee-equip:{}", uuid_string(target)) }
    } else {
        let mut items = inventory;
        items.resize(45, None);
        let name = name_of(ctx.ess, ctx.host, target);
        Container { rows: 5, title: escape_tags(&name), items, tag: format!("invsee:{}", uuid_string(target)) }
    };
    ctx.host.close_container(me);
    if ctx.host.open_container(me, &container) {
        st(ctx.ess).invsee.insert(me, target);
    }
    Ok(())
}

// ===== Jails ======================================================================================

fn load_jails(ess: &Ess) -> BTreeMap<String, Loc> {
    let yaml = ess.read_yaml("jail.yml");
    let mut out = BTreeMap::new();
    if let Some(Value::Object(map)) = yaml.get("jails") {
        for (name, v) in map {
            if let Some(loc) = Loc::from_yaml(v) {
                out.insert(name.to_lowercase(), loc);
            }
        }
    }
    out
}

/// Jails by lower-case name (Jails.getList), loaded from jail.yml on first use.
fn jails(ess: &mut Ess) -> &mut BTreeMap<String, Loc> {
    if st(ess).jails.is_none() {
        let loaded = load_jails(ess);
        st(ess).jails = Some(loaded);
    }
    st(ess).jails.get_or_insert_with(BTreeMap::new)
}

fn save_jails(ess: &mut Ess) {
    let mut out = serde_json::Map::new();
    for (name, loc) in jails(ess).clone() {
        let mut v = loc.to_yaml();
        v["world-name"] = json!(world_name(&loc.world));
        out.insert(name, v);
    }
    ess.write_yaml("jail.yml", &json!({ "jails": out }));
}

fn get_jail(ess: &mut Ess, name: &str) -> Result<Loc, CmdError> {
    jails(ess).get(&name.to_lowercase()).cloned().ok_or(CmdError::Tl("jailNotExist", vec![]))
}

/// Jails.sendToJail: teleports an online player (remembering /back) and records the jail.
fn send_to_jail(ess: &mut Ess, host: &mut dyn Host, player: Uuid, jail: &str) -> bool {
    let jail = jail.to_lowercase();
    let Some(loc) = jails(ess).get(&jail).cloned() else { return false };
    if let Some(here) = host.location(player) {
        ess.users.user(player).set("lastlocation", here.to_yaml());
        host.teleport(player, &loc);
    }
    ess.users.user(player).set("jail", json!(jail));
    true
}

enum FreePolicy {
    Back,
    Spawn,
    Off,
}

fn free_policy(ess: &Ess) -> FreePolicy {
    if let Some(v) = ess.config("teleport-back-when-freed-from-jail").and_then(Value::as_bool) {
        return if v { FreePolicy::Back } else { FreePolicy::Off };
    }
    match ess.config("teleport-when-freed") {
        Some(Value::Bool(false)) => FreePolicy::Off,
        Some(Value::String(s)) if s.eq_ignore_ascii_case("off") || s.eq_ignore_ascii_case("false") => FreePolicy::Off,
        Some(Value::String(s)) if s.eq_ignore_ascii_case("spawn") => FreePolicy::Spawn,
        _ => FreePolicy::Back,
    }
}

fn respawn_point(host: &mut dyn Host, player: Uuid) -> Option<Loc> {
    host.bed_spawn(player).or_else(|| host.world_spawn("minecraft:overworld"))
}

/// teleport-when-freed: back to where they were jailed, to spawn, or nowhere.
fn teleport_freed(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    let to = match free_policy(ess) {
        FreePolicy::Off => return,
        FreePolicy::Spawn => respawn_point(host, player),
        FreePolicy::Back => ess.users.get(player).and_then(|u| u.get("lastlocation").cloned()).and_then(|v| Loc::from_yaml(&v)).or_else(|| respawn_point(host, player)),
    };
    if let Some(to) = to {
        if let Some(here) = host.location(player) {
            ess.users.user(player).set("lastlocation", here.to_yaml());
        }
        host.teleport(player, &to);
    }
}

/// User.getFormattedJailTime.
fn formatted_jail_time(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    let now = host.now_millis();
    let online_jail = user_i64(ess, player, "timestamps.onlinejail");
    let until = if online_jail > 0 { (online_jail - host.statistic(player, PLAY_TIME)) * 50 + now } else { user_i64(ess, player, "timestamps.jail") };
    format_date_diff(ess, now, until)
}

/// User.checkJailTimeout: frees a player whose sentence is over.
fn check_jail_timeout(ess: &mut Ess, host: &mut dyn Host, player: Uuid, now: i64) -> bool {
    let timeout = user_i64(ess, player, "timestamps.jail");
    if timeout <= 0 {
        return false;
    }
    let online_jail = user_i64(ess, player, "timestamps.onlinejail");
    if online_jail > 0 && online_jail > host.statistic(player, PLAY_TIME) {
        return false;
    }
    if timeout < now && user_bool(ess, player, "jailed") {
        let u = ess.users.user(player);
        u.set("timestamps.jail", json!(0));
        u.set("timestamps.onlinejail", json!(0));
        u.set("jailed", json!(false));
        u.set("jail", Value::Null);
        ess.send_tl(host, Sender::Player(player), "haveBeenReleased", &[]);
        teleport_freed(ess, host, player);
        return true;
    }
    false
}

fn cmd_setjail(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let at = ctx.host.location(me).ok_or(CmdError::Quiet)?;
    jails(ctx.ess).insert(args[0].to_lowercase(), at);
    save_jails(ctx.ess);
    let shown: String = args[0].chars().filter(|c| !c.is_control()).collect();
    ctx.tl("jailSet", &[Arg::Plain(shown)]);
    Ok(())
}

fn cmd_deljail(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    get_jail(ctx.ess, &args[0])?;
    jails(ctx.ess).remove(&args[0].to_lowercase());
    save_jails(ctx.ess);
    ctx.tl("deleteJail", &[Arg::Plain(args[0].clone())]);
    Ok(())
}

fn cmd_jails(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let names: Vec<String> = jails(ctx.ess).keys().cloned().collect();
    if names.is_empty() {
        ctx.tl("noJailsDefined", &[]);
    } else {
        ctx.tl("jailList", &[Arg::Plain(names.join(" "))]);
    }
    Ok(())
}

fn cmd_jailedplayers(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let mut names = Vec::new();
    for u in ctx.ess.users.all() {
        if user_bool(ctx.ess, u, "jailed") {
            names.push(name_of(ctx.ess, ctx.host, u));
        }
    }
    if names.is_empty() {
        ctx.tl("jailedPlayersEmpty", &[]);
    } else {
        ctx.tl("jailedPlayersList", &[Arg::Plain(names.join(", "))]);
    }
    Ok(())
}

fn cmd_togglejail(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let player = find(ctx, &args[0], true, true)?;
    let name = name_of(ctx.ess, ctx.host, player);
    let online = is_online(ctx.host, player);
    let jailed = user_bool(ctx.ess, player, "jailed");
    let current = user_str(ctx.ess, player, "jail").unwrap_or_default();
    let online_time = ctx.ess.config_bool("jail-online-time", false);
    let now = ctx.host.now_millis();
    if !jailed {
        if !online && ctx.is_player() && !ctx.has("essentials.togglejail.offline") {
            ctx.tl("mayNotJailOffline", &[]);
            return Ok(());
        }
        if has_node(ctx.ess, ctx.host, player, "essentials.jail.exempt") {
            ctx.tl("mayNotJail", &[]);
            return Ok(());
        }
        let jail_name = if args.len() > 1 {
            Some(args[1].clone())
        } else if jails(ctx.ess).len() == 1 {
            jails(ctx.ess).keys().next().cloned()
        } else {
            None
        };
        if let Some(jail_name) = jail_name {
            get_jail(ctx.ess, &jail_name)?;
            let (display_time, time_diff) = if args.len() > 2 {
                let time = final_arg(args, 2);
                let display = parse_date_diff(&time, true, false, now).ok_or(CmdError::Tl("illegalDate", vec![]))?;
                let diff = parse_date_diff(&time, true, online_time, now).ok_or(CmdError::Tl("illegalDate", vec![]))?;
                (display, diff)
            } else {
                (0, 0)
            };
            if online {
                send_to_jail(ctx.ess, ctx.host, player, &jail_name);
            }
            let u = ctx.ess.users.user(player);
            u.set("jailed", json!(true));
            u.set("jail", json!(jail_name.to_lowercase()));
            if online {
                ctx.tl_to(Sender::Player(player), "userJailed", &[]);
            }
            if args.len() > 2 {
                let stat = ctx.host.statistic(player, PLAY_TIME);
                let u = ctx.ess.users.user(player);
                u.set("timestamps.jail", json!(time_diff));
                u.set("timestamps.onlinejail", json!(if online_time { stat + time_diff / 50 } else { 0 }));
            }
            let by = sender_name(ctx);
            let (key, a) = if time_diff > 0 {
                let time = format_date_diff(ctx.ess, now, display_time);
                ctx.tl("playerJailedFor", &[Arg::Plain(name.clone()), Arg::Plain(time.clone())]);
                ("jailNotifyJailedFor", vec![Arg::Plain(name), Arg::Plain(time), Arg::Plain(by)])
            } else {
                ctx.tl("playerJailed", &[Arg::Plain(name.clone())]);
                ("jailNotifyJailed", vec![Arg::Plain(name), Arg::Plain(by.clone()), Arg::Plain(by)])
            };
            log_tl(ctx.ess, ctx.host, key, &a);
            notify(ctx.ess, ctx.host, "essentials.jail.notify", key, &a);
            return Ok(());
        }
    }
    if args.len() >= 2 && jailed && !args[1].eq_ignore_ascii_case(&current) {
        ctx.tl("jailAlreadyIncarcerated", &[Arg::Plain(current)]);
        return Ok(());
    }
    if args.len() >= 2 && jailed {
        let time = final_arg(args, 2);
        let display = parse_date_diff(&time, true, false, now).ok_or(CmdError::Tl("illegalDate", vec![]))?;
        let diff = parse_date_diff(&time, true, online_time, now).ok_or(CmdError::Tl("illegalDate", vec![]))?;
        let stat = ctx.host.statistic(player, PLAY_TIME);
        let u = ctx.ess.users.user(player);
        u.set("timestamps.jail", json!(diff));
        u.set("timestamps.onlinejail", json!(if online_time { stat + diff / 50 } else { 0 }));
        let shown = format_date_diff(ctx.ess, now, display);
        ctx.tl("jailSentenceExtended", &[Arg::Plain(shown.clone())]);
        let by = sender_name(ctx);
        let a = [Arg::Plain(name), Arg::Plain(shown), Arg::Plain(by)];
        log_tl(ctx.ess, ctx.host, "jailNotifySentenceExtended", &a);
        notify(ctx.ess, ctx.host, "essentials.jail.notify", "jailNotifySentenceExtended", &a);
        return Ok(());
    }
    if args.len() == 1 {
        if !jailed {
            return Err(CmdError::Usage(None));
        }
        let u = ctx.ess.users.user(player);
        u.set("jailed", json!(false));
        u.set("timestamps.jail", json!(0));
        u.set("timestamps.onlinejail", json!(0));
        u.set("jail", Value::Null);
        if online {
            ctx.tl_to(Sender::Player(player), "jailReleasedPlayerNotify", &[]);
        }
        if online {
            let policy = free_policy(ctx.ess);
            teleport_freed(ctx.ess, ctx.host, player);
            if !matches!(policy, FreePolicy::Off) {
                ctx.tl("jailReleased", &[Arg::Plain(name)]);
            }
            return Ok(());
        }
        ctx.tl("jailReleased", &[Arg::Plain(name)]);
    }
    Ok(())
}

// ===== /essentials ================================================================================

const PLAYER_MOO: [&str; 7] = ["            (__)", "            (oo)", "   /------\\/", "  /  |      | |", " *  /\\---/\\", "    ~~    ~~", "....\"Have you mooed today?\"..."];
const CONSOLE_MOO: [&str; 7] = ["         (__)", "         (oo)", "   /------\\/", "  / |    ||", " *  /\\---/\\", "    ~~   ~~", "....\"Have you mooed today?\"..."];

fn remove_user(ess: &mut Ess, player: Uuid) {
    ess.users.loaded.remove(&player);
    let _ = std::fs::remove_file(ess.users.dir.join(format!("{}.yml", uuid_string(player))));
    ess.users.names.retain(|_, u| *u != player);
}

fn cmd_essentials(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Some(sub) = args.first() else { return Err(CmdError::Usage(None)) };
    let rest = &args[1..];
    match sub.as_str() {
        "reload" => {
            ctx.ess.reload();
            st(ctx.ess).jails = None;
            ctx.tl("essentialsReload", &[Arg::Plain(ESS_VERSION.into())]);
        }
        "version" | "ver" => {
            if ctx.is_player() && !ctx.has("essentials.version") {
                return Ok(());
            }
            ctx.tl("versionOutputFine", &[Arg::Plain("Server".into()), Arg::Plain("Horizon (Minecraft 26.3)".into())]);
            ctx.tl("versionOutputFine", &[Arg::Plain("Brand".into()), Arg::Plain("Horizon".into())]);
            ctx.tl("versionOutputFine", &[Arg::Plain("EssentialsX".into()), Arg::Plain(ESS_VERSION.into())]);
            let layer = if ctx.ess.config_bool("disable-eco", false) { "Disabled" } else { "Essentials (userdata)" };
            ctx.tl("versionOutputEconLayer", &[Arg::Plain(layer.into())]);
        }
        "cmd" | "commands" => ctx.tl("blockListEmpty", &[]),
        "debug" | "verbose" => {
            let current = st(ctx.ess).debug;
            let on = match rest.first().map(|a| a.to_lowercase()) {
                Some(a) if a == "on" || a == "true" => true,
                Some(a) if a == "off" || a == "false" => false,
                _ => !current,
            };
            st(ctx.ess).debug = on;
            ctx.send_mini(&format!("Essentials {ESS_VERSION} debug mode {}", if on { "enabled" } else { "disabled" }));
        }
        "cleanup" => {
            let days = rest.first().and_then(|d| d.parse::<i64>().ok());
            let Some(days) = days else {
                ctx.send_mini("This sub-command will delete users who haven't logged in in the last \\<days> days.");
                ctx.send_mini("Optional parameters define the minimum amount required to prevent deletion.");
                ctx.send_mini("Unless you define larger default values, this command will ignore people who have more than 0 money/homes.");
                return Err(CmdError::Msg(format!("/{} cleanup <days> [money] [homes]", ctx.label)));
            };
            ctx.tl("cleaning", &[]);
            let money_arg: f64 = rest.get(1).map(|m| m.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect::<String>()).and_then(|m| m.parse().ok()).unwrap_or(0.0);
            let homes_arg: usize = rest.get(2).and_then(|h| h.parse().ok()).unwrap_or(0);
            let now = ctx.host.now_millis();
            for u in ctx.ess.users.all() {
                let mut last = user_i64(ctx.ess, u, "timestamps.logout");
                if last == 0 {
                    last = user_i64(ctx.ess, u, "timestamps.login");
                }
                if last == 0 {
                    ctx.ess.users.user(u).set("timestamps.login", json!(now));
                    continue;
                }
                let homes = ctx.ess.users.get(u).and_then(|d| d.get("homes").and_then(Value::as_object).map(|h| h.len())).unwrap_or(0);
                let money = money_of(ctx.ess, u);
                if now - last < days * 86_400_000 || homes > homes_arg || money > money_arg {
                    continue;
                }
                remove_user(ctx.ess, u);
            }
            ctx.tl("cleaned", &[]);
        }
        "homes" => {
            let usage = "/<command> homes (fix | delete [world])".replace("<command>", &ctx.label);
            match rest.first().map(String::as_str) {
                Some("fix") => {
                    ctx.tl("fixingHomes", &[]);
                    let worlds = ctx.host.worlds();
                    for u in ctx.ess.users.all() {
                        let Some(Value::Object(homes)) = ctx.ess.users.get(u).and_then(|d| d.get("homes").cloned()) else { continue };
                        for (name, loc) in homes {
                            if Loc::from_yaml(&loc).is_none_or(|l| !worlds.contains(&l.world)) {
                                ctx.ess.users.user(u).set(&format!("homes.{name}"), Value::Null);
                            }
                        }
                    }
                    ctx.tl("fixedHomes", &[]);
                }
                Some("delete") => {
                    let world = rest.get(1).map(|w| world_id(w));
                    if let Some(w) = &world {
                        if !ctx.host.worlds().contains(w) {
                            return Err(CmdError::Tl("invalidWorld", vec![]));
                        }
                        ctx.tl("deletingHomesWorld", &[Arg::Plain(rest[1].clone())]);
                    } else {
                        ctx.tl("deletingHomes", &[]);
                    }
                    for u in ctx.ess.users.all() {
                        let Some(Value::Object(homes)) = ctx.ess.users.get(u).and_then(|d| d.get("homes").cloned()) else { continue };
                        for (name, loc) in homes {
                            if world.as_ref().is_none_or(|w| Loc::from_yaml(&loc).is_some_and(|l| l.world == *w)) {
                                ctx.ess.users.user(u).set(&format!("homes.{name}"), Value::Null);
                            }
                        }
                    }
                    match world {
                        Some(_) => ctx.tl("deletedHomesWorld", &[Arg::Plain(rest[1].clone())]),
                        None => ctx.tl("deletedHomes", &[]),
                    }
                }
                _ => {
                    ctx.send_mini("This sub-command provides a utility to mass-delete homes based on user options:");
                    ctx.send_mini("Use \"fix\" to delete all homes inside non-existent or unloaded worlds.");
                    ctx.send_mini("Use \"delete\" to delete all existing homes.");
                    ctx.send_mini("Use \"delete \\<worldname>\" to delete all homes inside a specific world.");
                    return Err(CmdError::Msg(usage));
                }
            }
        }
        "usermap" => {
            if !ctx.has("essentials.usermap") {
                return Ok(());
            }
            let max = ctx.ess.config_i64("max-user-cache-count", ctx.host.stats().memory_max_mb as i64);
            let (cached, total) = (ctx.ess.users.loaded.len(), ctx.ess.users.all().len());
            ctx.tl("usermapSize", &[Arg::from(cached), Arg::from(total), Arg::from(max)]);
            match rest.first().map(String::as_str) {
                Some("full") => {
                    for (name, u) in ctx.ess.users.names.clone() {
                        ctx.tl("usermapEntry", &[Arg::Plain(name), Arg::Plain(uuid_string(u))]);
                    }
                }
                Some("cache") => {
                    let names = ctx.ess.users.names.len();
                    ctx.tl("usermapKnown", &[Arg::from(total), Arg::from(names)]);
                }
                Some(other) => {
                    if let Some(u) = parse_uuid(other) {
                        for (name, v) in ctx.ess.users.names.clone() {
                            if v == u {
                                ctx.tl("usermapEntry", &[Arg::Plain(name), Arg::Plain(other.to_string())]);
                            }
                        }
                    } else if let Some(u) = ctx.ess.users.names.get(&other.to_lowercase()).copied() {
                        ctx.tl("usermapEntry", &[Arg::Plain(other.to_lowercase()), Arg::Plain(uuid_string(u))]);
                    }
                }
                None => {}
            }
        }
        "moo" => {
            if rest.first().is_some_and(|a| a == "moo") {
                for line in CONSOLE_MOO {
                    log_plain(ctx.ess, ctx.host, line);
                }
                for p in ctx.host.online() {
                    for line in PLAYER_MOO {
                        ctx.ess.send_mini(ctx.host, Sender::Player(p), &escape_tags(line));
                    }
                    let name = name_of(ctx.ess, ctx.host, p);
                    ctx.host.console_command(&format!("playsound minecraft:entity.cow.ambient master {name} ~ ~ ~ 1 1"));
                }
            } else if let Sender::Player(me) = ctx.sender {
                for line in PLAYER_MOO {
                    ctx.send_mini(&escape_tags(line));
                }
                let name = name_of(ctx.ess, ctx.host, me);
                ctx.host.console_command(&format!("execute as {name} at @s run playsound minecraft:entity.cow.ambient master @s ~ ~ ~ 1 1"));
            } else {
                for line in CONSOLE_MOO {
                    ctx.send_mini(&escape_tags(line));
                }
            }
        }
        "nyan" | "nya" => {
            const TUNE: &str = "1D#,1E,2F#,,2A#,1E,1D#,1E,2F#,2B,2D#,2E,2D#,2A#,2B,,2F#,,1D#,1E,2F#,2B,2C#,2A#,2B,2C#,2E,2D#,2E,2C#,,2F#,,2G#,,1D,1D#,,1C#,1D,1C#,1B,,1B,,1C#,,1D,,1D,1C#,1B,1C#,1D#,2F#,2G#,1D#,2F#,1C#,1D#,1B,1C#,1B,1D#,,2F#,,2G#,1D#,2F#,1C#,1D#,1B,1D,1D#,1D,1C#,1B,1C#,1D,,1B,1C#,1D#,2F#,1C#,1D,1C#,1B,1C#,,1B,,1C#";
            const NOTES: [(&str, f32); 24] = [
                ("1F#", 0.5), ("1G", 0.53), ("1G#", 0.56), ("1A", 0.6), ("1A#", 0.63), ("1B", 0.67), ("1C", 0.7), ("1C#", 0.76), ("1D", 0.8),
                ("1D#", 0.84), ("1E", 0.9), ("1F", 0.94), ("2F#", 1.0), ("2G", 1.06), ("2G#", 1.12), ("2A", 1.18), ("2A#", 1.26), ("2B", 1.34),
                ("2C", 1.42), ("2C#", 1.5), ("2D", 1.6), ("2D#", 1.68), ("2E", 1.78), ("2F", 1.88),
            ];
            for (i, note) in TUNE.split(',').enumerate() {
                let Some((_, pitch)) = NOTES.iter().find(|(n, _)| *n == note) else { continue };
                let line = format!("execute as @a at @s run playsound minecraft:block.note_block.harp master @s ~ ~ ~ 1 {pitch}");
                ctx.ess.scheduler.later(20 + 2 * i as u64, Box::new(move |_ess: &mut Ess, host: &mut dyn Host| {
                    host.console_command(&line);
                }));
            }
        }
        _ => return Err(CmdError::Usage(None)),
    }
    Ok(())
}

// ===== /backup ====================================================================================

/// Backup.run: save-all, save-off, the external command in the background, then save-on.
fn run_backup(ess: &mut Ess, host: &mut dyn Host) {
    if st(ess).backup.is_some() {
        return;
    }
    let command = ess.config_str("backup.command").unwrap_or_default();
    if command.trim().is_empty() {
        return;
    }
    if command.eq_ignore_ascii_case("save-all") {
        host.console_command("save-all");
        return;
    }
    log_tl(ess, host, "backupStarted", &[]);
    host.console_command("save-all");
    host.console_command("save-off");
    let done = Arc::new(AtomicBool::new(false));
    st(ess).backup = Some(done.clone());
    let dir = host.server_dir();
    let parts: Vec<String> = command.split(' ').map(str::to_string).collect();
    std::thread::spawn(move || {
        let _ = std::process::Command::new(&parts[0]).args(&parts[1..]).current_dir(dir).output();
        done.store(true, Ordering::SeqCst);
    });
}

fn cmd_backup(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let command = ctx.ess.config_str("backup.command").unwrap_or_default();
    if command.trim().is_empty() || command.eq_ignore_ascii_case("save-all") {
        return Err(CmdError::Tl("backupDisabled", vec![]));
    }
    run_backup(ctx.ess, ctx.host);
    ctx.tl("backupStarted", &[]);
    Ok(())
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
        assert_eq!(f(NOW + 367 * DAY), "1 year 1 day");
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
        // Offline players' exemption counts too.
        t.quit(bob);
        assert_eq!(t.cmd(carol, "/tempban Bob 1h"), ["You may not tempban that player."]);
        let dave = t.join("Dave");
        t.quit(dave);
        assert_eq!(t.cmd(carol, "/tempban Dave 2h"), ["You may not ban a player for this period of time."]);
        assert_eq!(t.cmd(carol, "/tempban Dave 30m"), none());
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
            ["Command Help: /sudo", "Description: Make another user perform a command.", "Usage(s);", "/sudo <player> sudo [args] - Makes the specified player run the given command"]
        );
    }

    #[test]
    fn sudo_exempt_only_for_players() {
        let (mut t, alice, bob) = server();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.ping]\nusers:\n  bob:\n    permissions: [essentials.sudo.exempt]\n");
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
        let mut t = TestServer::with_config("list:\n  Staff: mod helper\n");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.list]\n  mod: {}\n  helper: {}\nusers:\n  alice:\n    groups: [mod]\n  bob:\n    groups: [helper]\n");
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
        assert_eq!(t.cmd(alice, "/near Bob 9"), ["Players nearby: Carol(8m)"]);
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
        // Like EssentialsX, /unjail on a free player with a single jail jails them.
        assert_eq!(t.console("unjail Bob"), ["Player Bob jailed."]);
    }

    #[test]
    fn jail_allow_nodes_and_exempt() {
        let (mut t, _alice, bob, _cell) = jail_server("");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.ping, essentials.jail.allow.ping, essentials.jail.allow-break]\n");
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
        assert_eq!(logs(&mut t).last().unwrap(), "PlayerBob jail's time extended to 1 hour by CONSOLE.");
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
        let mut t = TestServer::with_config("command-cooldowns:\n  ping: 60\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/ping"), ["Pong!"]);
        assert_eq!(t.cmd(bob, "/ping"), ["You cannot type that command for 1 minute."]);
        assert_eq!(t.cmd(bob, "/eping hi"), ["You cannot type that command for 1 minute."]);
        let data = t.userdata(bob);
        let saved = data["timestamps"]["command-cooldowns"].as_array().unwrap().clone();
        assert_eq!(saved[0]["pattern"], json!("ping( .*)?"));
        assert_eq!(saved[0]["value"], json!(NOW + 60_000));
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
    fn command_cooldowns_with_regex_entries() {
        let mut t = TestServer::with_config("command-cooldowns:\n  '^p.ng( .*)?': 30\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/echo a"), ["a"]);
        assert_eq!(t.cmd(bob, "/pong"), ["You cannot type that command for 30 seconds."]);
        assert_eq!(t.userdata(bob)["timestamps"]["command-cooldowns"][0]["pattern"], json!("p.ng( .*)?"));
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
        assert_eq!(t.console("essentials usermap cache"), ["Current cached users in user map is 2/2/2,048.", "There are 2 known users to the user cache with 2 name to UUID pairs."]);
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
        assert_eq!(logs(&mut t), ["CONSOLE issued server command: /backup ", "Backup started."]);
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
