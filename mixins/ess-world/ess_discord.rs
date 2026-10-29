// horizon-essentials area `world`, module `ess_discord` (pure logic, standalone-tested): EssentialsX Discord +
// DiscordLink over Discord's REST API only (no gateway): MC->Discord through the bot (or a webhook for player
// names/avatars), Discord->MC by polling the configured channels, message commands standing in for the slash
// commands, link codes, account storage and role sync.
//
// Requests go through Host::http_request (the glue runs them off the server thread); the world area's tick
// dispatcher polls Host::http_poll once and hands each answer to `on_http`, which claims the ids tagged HTTP_ID.
//
// Message commands: a message in a mapped channel that starts with `message-command-prefix` (horizon addition,
// default "/") followed by execute, msg, list (EssentialsX Discord) or link, unlink, account (DiscordLink) runs the
// same logic as the slash command, with the same `commands.<name>.enabled/allowed-roles/admin-roles` checks.
// Replies are channel messages (Discord has no ephemeral replies without interactions); `hide-command` deletes
// the command message instead (always for link/unlink/account, so link codes don't stay visible).

use crate::game::ess_core::*;
use crate::game::ess_text::{escape_tags, legacy_to_mini, plain, strip_legacy};
use crate::game::{ess_defaults, ess_yaml};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub const API: &str = "https://discord.com/api/v10";
pub const ADVANCED_RELAY_NAME: &str = "EssX Advanced Relay";
/// The tag in the high bits of this module's HTTP request ids ("DI").
pub const HTTP_ID: u64 = 0x4449 << 48;
const HTTP_TAG_MASK: u64 = 0xffff << 48;
/// Discord channels are polled every 5 s.
const POLL_TICKS: u64 = 100;
/// Guild members (nickname, roles) are cached 5 min.
const MEMBER_TTL: u64 = 6000;
const PLAYER_TYPES: [&str; 12] = ["join", "first-join", "leave", "chat", "private-chat", "death", "afk", "advancement", "action", "local", "question", "shout"];

// ===== State ======================================================================================

#[derive(Clone, Debug)]
enum Kind {
    Send,
    SelfUser,
    Roles,
    Channel(String),
    Poll(String),
    Webhooks(String),
    CreateWebhook(String),
    Member(String),
    DmChannel(String),
    Other,
}

#[derive(Clone, Debug)]
struct Req {
    method: &'static str,
    url: String,
    body: Option<Value>,
    kind: Kind,
}

#[derive(Clone, Debug, Default)]
struct Member {
    nick: Option<String>,
    roles: Vec<String>,
    fetched: u64,
    missing: bool,
}

#[derive(Clone, Debug)]
struct Role {
    name: String,
    color: u32,
    position: i64,
    admin: bool,
}

#[derive(Default)]
pub struct Discord {
    loaded: bool,
    config: Value,
    link: Value,
    active: bool,
    /// Why the module is inactive (a message key).
    error: &'static str,
    started: bool,
    began: Option<std::time::Instant>,
    tick: u64,
    next_id: u64,
    pending: HashMap<u64, Req>,
    queue: VecDeque<Req>,
    blocked_until: u64,
    self_id: Option<String>,
    bot_name: String,
    roles: HashMap<String, Role>,
    channel_names: HashMap<String, String>,
    /// channel id -> newest message id seen (None until the first poll primed it).
    last_seen: HashMap<String, Option<String>>,
    polling: HashSet<String>,
    webhooks: HashMap<String, (String, String)>,
    members: HashMap<String, Member>,
    member_pending: HashSet<String>,
    waiting: HashMap<String, Vec<(String, Value)>>,
    /// discord id -> (player, unsync) waiting for the member.
    sync_waiting: HashMap<String, Vec<(Uuid, bool)>>,
    codes: BTreeMap<String, Uuid>,
    /// uuid string -> discord id (discordlink/accounts.json).
    accounts: BTreeMap<String, String>,
    rng: u64,
    sync_cursor: usize,
}

fn dc(ess: &mut Ess) -> &mut Discord {
    if !ess.state::<Discord>().loaded {
        load(ess);
    }
    ess.state::<Discord>()
}

/// A module config: the default file shipped by EssentialsX, overlaid by the one on disk.
fn layered(ess: &Ess, name: &str) -> Value {
    let mut base = ess_defaults::file(name).and_then(|t| ess_yaml::parse(t).ok()).unwrap_or(json!({}));
    if let Some(Value::Object(file)) = std::fs::read_to_string(ess.dir.join(name)).ok().and_then(|t| ess_yaml::parse(&t).ok()) {
        merge(&mut base, Value::Object(file));
    }
    base
}

fn load(ess: &mut Ess) {
    let config = layered(ess, "discord/config.yml");
    let link = layered(ess, "discordlink/config.yml");
    let accounts: BTreeMap<String, String> = std::fs::read_to_string(ess.dir.join("discordlink/accounts.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let token = ess_yaml::get(&config, "token").map(str_of).unwrap_or_default();
    let guild = ess_yaml::get(&config, "guild").and_then(id_of);
    let (active, error) = if token.replace("INSERT-TOKEN-HERE", "").trim().is_empty() {
        (false, "discordErrorNoToken")
    } else if guild.is_none() {
        (false, "discordErrorNoGuild")
    } else {
        (true, "")
    };
    *ess.state::<Discord>() = Discord { loaded: true, config, link, active, error, accounts, began: Some(std::time::Instant::now()), ..Discord::default() };
}

/// Re-reads discord/config.yml, discordlink/config.yml and the linked accounts (a restart of the module).
pub fn reload(ess: &mut Ess) {
    load(ess);
}

fn str_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// A Discord snowflake from YAML/JSON (numbers or strings); 0 means "not set".
fn id_of(v: &Value) -> Option<String> {
    let s = str_of(v);
    (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) && s.chars().any(|c| c != '0')).then_some(s)
}

fn cfg(ess: &mut Ess, path: &str) -> Option<Value> {
    ess_yaml::get(&dc(ess).config, path).cloned()
}
fn cfg_str(ess: &mut Ess, path: &str, default: &str) -> String {
    cfg(ess, path).filter(|v| !v.is_null()).map(|v| str_of(&v)).unwrap_or_else(|| default.to_string())
}
fn cfg_bool(ess: &mut Ess, path: &str, default: bool) -> bool {
    cfg(ess, path).and_then(|v| v.as_bool()).unwrap_or(default)
}
fn cfg_list(ess: &mut Ess, path: &str) -> Vec<String> {
    match cfg(ess, path) {
        Some(Value::Array(a)) => a.iter().map(str_of).filter(|s| !s.is_empty()).collect(),
        Some(Value::String(s)) if !s.is_empty() => vec![s],
        _ => Vec::new(),
    }
}
fn link_cfg(ess: &mut Ess, path: &str) -> Option<Value> {
    ess_yaml::get(&dc(ess).link, path).cloned()
}
fn link_bool(ess: &mut Ess, path: &str, default: bool) -> bool {
    link_cfg(ess, path).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn guild(ess: &mut Ess) -> String {
    cfg(ess, "guild").as_ref().and_then(id_of).unwrap_or_default()
}

/// `channels:` name -> id (unset ids skipped).
fn channel_map(ess: &mut Ess) -> Vec<(String, String)> {
    match cfg(ess, "channels") {
        Some(Value::Object(m)) => m.iter().filter_map(|(k, v)| id_of(v).map(|id| (k.clone(), id))).collect(),
        _ => Vec::new(),
    }
}

/// DiscordSettings.getChannelId: a raw id, or a name from `channels:`.
fn channel_id(ess: &mut Ess, key: &str) -> Option<String> {
    if !key.is_empty() && key.chars().all(|c| c.is_ascii_digit()) {
        return id_of(&json!(key));
    }
    channel_map(ess).into_iter().find(|(k, _)| k == key).map(|(_, id)| id)
}

fn channel_keys(ess: &mut Ess, id: &str) -> Vec<String> {
    channel_map(ess).into_iter().filter(|(_, c)| c == id).map(|(k, _)| k).collect()
}

/// JDADiscordService.getChannel(type, primaryFallback = true).
fn channel_for_type(ess: &mut Ess, key: &str) -> Option<String> {
    let def = if key.chars().all(|c| c.is_ascii_digit()) { key.to_string() } else { cfg_str(ess, &format!("message-types.{key}"), "none") };
    channel_id(ess, &def).or_else(|| channel_id(ess, "primary"))
}

fn channel_name(ess: &mut Ess, id: &str) -> String {
    if let Some(n) = dc(ess).channel_names.get(id) {
        return n.clone();
    }
    channel_keys(ess, id).into_iter().next().unwrap_or_else(|| id.to_string())
}

fn use_webhooks(ess: &mut Ess) -> bool {
    let named = cfg_bool(ess, "show-name", false) || cfg_bool(ess, "show-displayname", false);
    let format = cfg(ess, "messages.mc-to-discord-name-format").map(|v| str_of(&v));
    cfg_bool(ess, "show-avatar", false) || named || format.is_some_and(|f| !f.is_empty() && f != "{botname}")
}

// ===== Text helpers ===============================================================================

/// MessageUtil.sanitizeDiscordMarkdown.
pub fn sanitize(s: &str) -> String {
    s.replace('*', "\\*").replace('~', "\\~").replace('_', "\\_").replace('`', "\\`").replace('>', "\\>").replace('|', "\\|")
}

/// FormatUtil.replaceFormat: `&` codes become `§` codes.
fn replace_format(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if *c == '&' && chars.get(i + 1).is_some_and(|n| "0123456789abcdefklmnorxABCDEFKLMNORX".contains(*n)) {
            out.push('§');
        } else {
            out.push(*c);
        }
    }
    out
}

/// A config format with `{name}` / `{NAME}` placeholders (DiscordSettings.generateMessageFormat), one pass.
fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) if args.iter().any(|(k, _)| *k == &after[..j] || k.to_uppercase() == after[..j]) => {
                let key = &after[..j];
                let value = args.iter().find(|(k, _)| *k == key || k.to_uppercase() == key).map(|(_, v)| v.clone()).unwrap_or_default();
                out.push_str(&value);
                rest = &after[j + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn format(ess: &mut Ess, key: &str, default: &str, args: &[(&str, String)]) -> String {
    let template = cfg(ess, &format!("messages.{key}")).filter(|v| !v.is_null()).map(|v| str_of(&v)).unwrap_or_else(|| default.to_string());
    fill(&strip_legacy(&template, true), args)
}

/// A translated message as Discord text (no styles).
fn tl_plain(ess: &Ess, key: &str, args: &[&str]) -> String {
    let args: Vec<Arg> = args.iter().map(|a| Arg::Plain(a.to_string())).collect();
    ess.tl(key, &args).split('\n').map(|line| plain(&ess.render(line))).collect::<Vec<_>>().join("\n")
}

fn abbreviate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max || max < 4 {
        return s.to_string();
    }
    format!("{}...", s.chars().take(max - 3).collect::<String>())
}

fn player_name(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    host.name(player).or_else(|| ess.users.get(player).and_then(|u| u.name())).unwrap_or_else(|| uuid_string(player))
}

// ===== Requests ===================================================================================

fn request(ess: &mut Ess, method: &'static str, url: String, body: Option<Value>, kind: Kind) {
    dc(ess).queue.push_back(Req { method, url, body, kind });
}

fn send_req(ess: &mut Ess, host: &mut dyn Host, req: Req) {
    let token = cfg_str(ess, "token", "");
    let d = dc(ess);
    d.next_id += 1;
    let id = HTTP_ID | d.next_id;
    let mut headers = vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        ("User-Agent".to_string(), "DiscordBot (https://essentialsx.net, 2.21)".to_string()),
    ];
    if !req.url.contains("/webhooks/") || req.method == "GET" {
        headers.push(("Authorization".to_string(), format!("Bot {token}")));
    }
    let body = req.body.as_ref().map(Value::to_string);
    host.http_request(id, req.method, &req.url, &headers, body.as_deref());
    d.pending.insert(id, req);
}

fn flush(ess: &mut Ess, host: &mut dyn Host, force: bool) {
    loop {
        let d = dc(ess);
        if !d.active || (!force && d.tick < d.blocked_until) {
            return;
        }
        let Some(req) = d.queue.pop_front() else { return };
        send_req(ess, host, req);
    }
}

// ===== MC -> Discord ==============================================================================

/// DiscordUtil.dispatchDiscordMessage + JDADiscordService.sendMessage: posts `message` for a message type
/// (join, chat, kick...) to its channel, through the relay webhook when it carries a player's name/avatar.
pub fn send_message(ess: &mut Ess, key: &str, message: &str, allow_ping: bool, avatar: Option<String>, name: Option<String>, player: Option<Uuid>) {
    if !dc(ess).active || cfg_str(ess, &format!("message-types.{key}"), "none").eq_ignore_ascii_case("none") {
        return;
    }
    if key == "chat" && link_bool(ess, "block-unlinked-chat", false) && player.is_some_and(|p| linked_discord_id(ess, p).is_none()) {
        return;
    }
    let Some(channel) = channel_for_type(ess, key) else { return };
    let content = strip_legacy(message, false);
    let webhook = if PLAYER_TYPES.contains(&key) { dc(ess).webhooks.get(&channel).cloned() } else { None };
    if let Some((id, token)) = webhook {
        let name = name.filter(|n| !n.is_empty()).unwrap_or_else(|| dc(ess).bot_name.clone());
        let mentions = if allow_ping { json!({"parse": ["users", "roles", "everyone"]}) } else { json!({"parse": ["users"]}) };
        let mut body = json!({"content": content, "username": strip_legacy(&name, false), "allowed_mentions": mentions});
        if let Some(a) = avatar {
            body["avatar_url"] = json!(a);
        }
        request(ess, "POST", format!("{API}/webhooks/{id}/{token}"), Some(body), Kind::Send);
        return;
    }
    let mut body = json!({"content": content});
    if !allow_ping {
        body["allowed_mentions"] = json!({"parse": ["users"]});
    }
    request(ess, "POST", format!("{API}/channels/{channel}/messages"), Some(body), Kind::Send);
}

fn avatar_of(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    let name = player_name(ess, host, player);
    cfg_str(ess, "avatar-url", "https://crafthead.net/helm/{uuid}").replace("{uuid}", &uuid_string(player)).replace("{name}", &name)
}

/// Player-related relay: name format (webhook username), avatar, ping permission.
fn send_player(ess: &mut Ess, host: &mut dyn Host, key: &str, message: &str, player: Uuid) {
    let name = player_name(ess, host, player);
    let world = host.location(player).map(|l| world_name(&l.world)).unwrap_or_default();
    let (prefix, suffix) = affixes(ess, host, player);
    let legacy = if cfg_bool(ess, "show-displayname", false) {
        Some("{displayname}")
    } else if cfg_bool(ess, "show-name", false) {
        Some("{username}")
    } else {
        None
    };
    let template = cfg(ess, "messages.mc-to-discord-name-format").filter(|v| !v.is_null()).map(|v| str_of(&v)).or(legacy.map(str::to_string)).unwrap_or_else(|| "{botname}".into());
    let bot = dc(ess).bot_name.clone();
    let formatted_name = fill(&strip_legacy(&template, true), &[("username", name.clone()), ("displayname", name), ("world", world), ("prefix", prefix), ("suffix", suffix), ("botname", bot)]);
    let avatar = avatar_of(ess, host, player);
    let ping = ess.has(host, Sender::Player(player), "essentials.discord.ping");
    send_message(ess, key, message, ping, Some(avatar), Some(formatted_name), Some(player));
}

fn world_name(id: &str) -> String {
    match id {
        "minecraft:overworld" => "world".into(),
        "minecraft:the_nether" => "world_nether".into(),
        "minecraft:the_end" => "world_the_end".into(),
        other => other.trim_start_matches("minecraft:").into(),
    }
}

fn affixes(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> (String, String) {
    let group = ess.group(host, player);
    let g = ess.perms.groups.get(&group).cloned().unwrap_or_default();
    (sanitize(&strip_legacy(&g.prefix, true)), sanitize(&strip_legacy(&g.suffix, true)))
}

fn vanish_hidden(ess: &mut Ess, player: Uuid) -> bool {
    cfg_bool(ess, "vanish-hide-messages", true) && ess.is_vanished(player)
}

fn on_chat(ess: &mut Ess, host: &mut dyn Host, player: Uuid, message: &str) -> Chat {
    if !dc(ess).active {
        return Chat::Pass;
    }
    if frozen(ess, player) {
        prompt(ess, host, player);
        return Chat::Cancel;
    }
    if ess.users.get(player).is_some_and(|u| u.bool("muted")) {
        return Chat::Pass;
    }
    let (key, fmt_key, default, text) = if ess.config_i64("chat.radius", 0) > 0 {
        if let Some(t) = message.strip_prefix('!').filter(|t| !t.is_empty()) {
            ("shout", "mc-to-discord-shout", "**[Shout]** {displayname}: {message}", t)
        } else if let Some(t) = message.strip_prefix('?').filter(|t| !t.is_empty()) {
            ("question", "mc-to-discord-question", "**[Question]** {displayname}: {message}", t)
        } else {
            ("local", "mc-to-discord-local", "**[Local]** {displayname}: {message}", message)
        }
    } else {
        ("chat", "mc-to-discord", "{displayname}: {message}", message)
    };
    let name = sanitize(&player_name(ess, host, player));
    let markdown = ess.has(host, Sender::Player(player), "essentials.discord.markdown");
    let text = if markdown { text.to_string() } else { sanitize(text) };
    let world = sanitize(&host.location(player).map(|l| world_name(&l.world)).unwrap_or_default());
    let (prefix, suffix) = affixes(ess, host, player);
    let line = format(ess, fmt_key, default, &[("username", name.clone()), ("displayname", name), ("message", text), ("world", world), ("prefix", prefix), ("suffix", suffix)]);
    send_player(ess, host, key, &line, player);
    Chat::Pass
}

fn on_join(ess: &mut Ess, host: &mut dyn Host, player: Uuid, first: bool) {
    if !dc(ess).active {
        return;
    }
    match link_policy(ess).as_str() {
        "kick" if linked_discord_id(ess, player).is_none() => {
            let code = code_for(ess, host, player);
            let url = cfg_str(ess, "discord-url", "https://discord.gg/invite-code");
            let text = ess.tl("discordLinkLoginKick", &[Arg::Plain(format!("/link {code}")), Arg::Plain(url)]);
            let spans = ess.render(&text.replace('\n', "<newline>"));
            host.kick(player, &spans);
            return;
        }
        "freeze" if linked_discord_id(ess, player).is_none() => prompt(ess, host, player),
        _ => {}
    }
    if let Some(id) = linked_discord_id(ess, player) {
        sync_roles(ess, player, &id, false);
    }
    let silent = ess.config_bool("allow-silent-join-quit", false) && ess.has(host, Sender::Player(player), "essentials.silentjoin");
    if silent || vanish_hidden(ess, player) {
        return;
    }
    let name = player_name(ess, host, player);
    let online = host.online().len();
    let unique = ess.users.all().len();
    let (key, default) = if first {
        ("first-join", ":arrow_right: :first_place: {displayname} has joined the server for the first time!")
    } else {
        ("join", ":arrow_right: {displayname} has joined!")
    };
    let line = format(ess, key, default, &[("username", sanitize(&name)), ("displayname", sanitize(&name)), ("joinmessage", sanitize(&format!("{name} joined the game"))), ("online", online.to_string()), ("unique", unique.to_string())]);
    send_player(ess, host, key, &line, player);
}

fn on_quit(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    if !dc(ess).active {
        return;
    }
    let silent = ess.config_bool("allow-silent-join-quit", false) && ess.has(host, Sender::Player(player), "essentials.silentquit");
    if silent || vanish_hidden(ess, player) {
        return;
    }
    let name = player_name(ess, host, player);
    let online = host.online().into_iter().filter(|p| *p != player).count();
    let unique = ess.users.all().len();
    let line = format(ess, "quit", ":arrow_left: {displayname} has left!", &[("username", sanitize(&name)), ("displayname", sanitize(&name)), ("quitmessage", sanitize(&format!("{name} left the game"))), ("online", online.to_string()), ("unique", unique.to_string())]);
    send_player(ess, host, "leave", &line, player);
}

fn on_death(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _at: &Loc) {
    if !dc(ess).active || vanish_hidden(ess, player) {
        return;
    }
    let name = sanitize(&player_name(ess, host, player));
    // The core's death event carries no vanilla death message: "<name> died" stands in for it.
    let line = format(ess, "death", ":skull: {deathmessage}", &[("username", name.clone()), ("displayname", name.clone()), ("deathmessage", format!("{name} died"))]);
    send_player(ess, host, "death", &line, player);
}

/// AfkStatusChangeEvent (for the social area's /afk).
pub fn relay_afk(ess: &mut Ess, host: &mut dyn Host, player: Uuid, afk: bool) {
    if !dc(ess).active || vanish_hidden(ess, player) {
        return;
    }
    let name = sanitize(&player_name(ess, host, player));
    let (key, default) = if afk { ("afk", ":person_walking: {displayname} is now AFK!") } else { ("un-afk", ":keyboard: {displayname} is no longer AFK!") };
    let line = format(ess, key, default, &[("username", name.clone()), ("displayname", name)]);
    send_player(ess, host, "afk", &line, player);
}

/// UserActionEvent (/me).
pub fn relay_action(ess: &mut Ess, host: &mut dyn Host, player: Uuid, action: &str) {
    if !dc(ess).active || vanish_hidden(ess, player) {
        return;
    }
    let name = sanitize(&player_name(ess, host, player));
    let line = format(ess, "action", ":person_biking: {displayname} *{action}*", &[("username", name.clone()), ("displayname", name), ("action", action.to_string())]);
    send_player(ess, host, "action", &line, player);
}

/// PlayerKickEvent (the admin area's /kick, /kickall, bans).
pub fn relay_kick(ess: &mut Ess, host: &mut dyn Host, player: Uuid, reason: &str) {
    if !dc(ess).active || vanish_hidden(ess, player) {
        return;
    }
    let name = sanitize(&player_name(ess, host, player));
    let line = format(ess, "kick", "{displayname} was kicked with reason: {reason}", &[("username", name.clone()), ("displayname", name), ("reason", sanitize(reason))]);
    send_message(ess, "kick", &line, false, None, None, Some(player));
}

/// An advancement completed (needs a vanilla advancement hook).
pub fn relay_advancement(ess: &mut Ess, host: &mut dyn Host, player: Uuid, advancement: &str) {
    if !dc(ess).active || vanish_hidden(ess, player) {
        return;
    }
    let name = sanitize(&player_name(ess, host, player));
    let line = format(ess, "advancement", ":medal: {displayname} has completed the advancement **{advancement}**!", &[("username", name.clone()), ("displayname", name), ("advancement", advancement.to_string())]);
    send_player(ess, host, "advancement", &line, player);
}

/// PrivateMessageSentEvent (/msg, /r), unless the sender has essentials.chat.spy.exempt.
pub fn relay_private(ess: &mut Ess, host: &mut dyn Host, sender: Uuid, to: &str, message: &str) {
    if !dc(ess).active || ess.has(host, Sender::Player(sender), "essentials.chat.spy.exempt") {
        return;
    }
    let from = sanitize(&player_name(ess, host, sender));
    let line = format(ess, "private-chat", "[SocialSpy] {sender-username} -> {receiver-username}: {message}", &[("sender-username", from.clone()), ("sender-displayname", from), ("receiver-username", sanitize(to)), ("receiver-displayname", sanitize(to)), ("message", sanitize(message))]);
    send_player(ess, host, "private-chat", &line, sender);
}

/// MuteStatusChangeEvent: `until` None = permanent; `controller` None = the console.
pub fn relay_mute(ess: &mut Ess, host: &mut dyn Host, player: Uuid, muted: bool, controller: Option<&str>, until: Option<&str>, reason: Option<&str>) {
    if !dc(ess).active {
        return;
    }
    let name = sanitize(&player_name(ess, host, player));
    let ctrl = sanitize(controller.unwrap_or("Console"));
    let mut args = vec![("username", name.clone()), ("displayname", name), ("controllername", ctrl.clone()), ("controllerdisplayname", ctrl)];
    let line = match (muted, until, reason) {
        (false, _, _) => format(ess, "unmute", "{displayname} unmuted.", &args),
        (true, Some(t), r) => {
            args.push(("time", t.to_string()));
            args.push(("reason", sanitize(r.unwrap_or(""))));
            if r.is_some() {
                format(ess, "temporary-mute-reason", "{controllerdisplayname} has muted player {displayname} for {time}. Reason: {reason}.", &args)
            } else {
                format(ess, "temporary-mute", "{controllerdisplayname} has muted player {displayname} for {time}.", &args)
            }
        }
        (true, None, r) => {
            args.push(("reason", sanitize(r.unwrap_or(""))));
            if r.is_some() {
                format(ess, "permanent-mute-reason", "{controllerdisplayname} has muted player {displayname}. Reason: {reason}.", &args)
            } else {
                format(ess, "permanent-mute", "{controllerdisplayname} permanently muted {displayname}.", &args)
            }
        }
    };
    send_message(ess, "mute", &line, false, None, None, Some(player));
}

/// The server is stopping: sent at once (the glue calls this from its shutdown path).
pub fn on_server_stop(ess: &mut Ess, host: &mut dyn Host) {
    if !dc(ess).active {
        return;
    }
    let line = cfg_str(ess, "messages.server-stop", ":octagonal_sign: The server has stopped!");
    send_message(ess, "server-stop", &line, true, None, None, None);
    flush(ess, host, true);
}

/// LinkBukkitListener.onMail: new mail is sent to the recipient's linked Discord account as a DM.
pub fn relay_mail(ess: &mut Ess, _host: &mut dyn Host, to: Uuid, from: &str, message: &str) {
    if !dc(ess).active || !link_bool(ess, "relay-mail", true) {
        return;
    }
    let Some(id) = linked_discord_id(ess, to) else { return };
    let content = tl_plain(ess, "discordMailLine", &[&sanitize(from), &sanitize(&strip_legacy(message, true))]);
    request(ess, "POST", format!("{API}/users/@me/channels"), Some(json!({"recipient_id": id})), Kind::DmChannel(content));
}

// ===== Tick: startup, polling, answers ============================================================

fn on_tick(ess: &mut Ess, host: &mut dyn Host) {
    if !dc(ess).active {
        return;
    }
    dc(ess).tick += 1;
    if !dc(ess).started {
        startup(ess, host);
    }
    let tick = dc(ess).tick;
    if tick % POLL_TICKS == 0 {
        for channel in poll_channels(ess) {
            let d = dc(ess);
            if d.polling.contains(&channel) {
                continue;
            }
            d.polling.insert(channel.clone());
            let url = match d.last_seen.get(&channel).cloned().flatten() {
                Some(last) => format!("{API}/channels/{channel}/messages?after={last}&limit=50"),
                None => format!("{API}/channels/{channel}/messages?limit=1"),
            };
            request(ess, "GET", url, None, Kind::Poll(channel));
        }
    }
    let resync = link_cfg(ess, "role-sync.resync-delay").and_then(|v| v.as_u64()).unwrap_or(5).max(1) * 1200;
    if tick % resync == 0 {
        resync_batch(ess);
    }
    flush(ess, host, false);
}

/// An HTTP answer from the host: handled and true when the id is one of this module's requests.
pub fn on_http(ess: &mut Ess, host: &mut dyn Host, id: u64, status: u16, body: &str) -> bool {
    if id & HTTP_TAG_MASK != HTTP_ID {
        return false;
    }
    if let Some(req) = dc(ess).pending.remove(&id) {
        handle(ess, host, req, status, body);
    }
    true
}

fn startup(ess: &mut Ess, host: &mut dyn Host) {
    dc(ess).started = true;
    let guild = guild(ess);
    request(ess, "GET", format!("{API}/users/@me"), None, Kind::SelfUser);
    request(ess, "GET", format!("{API}/guilds/{guild}/roles"), None, Kind::Roles);
    let mut seen = Vec::new();
    for (_, id) in channel_map(ess) {
        if !seen.contains(&id) {
            seen.push(id.clone());
            request(ess, "GET", format!("{API}/channels/{id}"), None, Kind::Channel(id));
        }
    }
    if use_webhooks(ess) {
        let mut hooked = Vec::new();
        for key in PLAYER_TYPES {
            if let Some(c) = channel_for_type(ess, key) {
                if !hooked.contains(&c) {
                    hooked.push(c.clone());
                    request(ess, "GET", format!("{API}/channels/{c}/webhooks"), None, Kind::Webhooks(c));
                }
            }
        }
    }
    let secs = dc(ess).began.map_or(0.0, |b| b.elapsed().as_secs_f32());
    let line = format(ess, "server-start", ":white_check_mark: The server has started in {starttimeseconds} seconds!", &[("starttimeseconds", format!("{secs:.2}"))]);
    send_message(ess, "server-start", &line, true, None, None, None);
    let _ = host;
}

/// Every channel with a name in `channels:`, plus the console channel for command relay.
fn poll_channels(ess: &mut Ess) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, id) in channel_map(ess) {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    if let Some(c) = console_channel(ess) {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

fn console_channel(ess: &mut Ess) -> Option<String> {
    if !cfg_bool(ess, "console.command-relay", false) {
        return None;
    }
    let def = cfg_str(ess, "console.channel", "none");
    channel_id(ess, &def)
}

fn handle(ess: &mut Ess, host: &mut dyn Host, req: Req, status: u16, body: &str) {
    if status == 429 {
        let retry = serde_json::from_str::<Value>(body).ok().and_then(|v| v["retry_after"].as_f64()).unwrap_or(1.0);
        let d = dc(ess);
        d.blocked_until = d.tick + (retry * 20.0).ceil() as u64;
        if let Kind::Poll(c) = &req.kind {
            d.polling.remove(c);
            return;
        }
        d.queue.push_front(req);
        return;
    }
    let ok = (200..300).contains(&status);
    let json: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    match req.kind {
        Kind::SelfUser => {
            if !ok {
                let reason = json["message"].as_str().map(str::to_string).unwrap_or_else(|| format!("HTTP {status}"));
                let msg = ess.tl("discordErrorLogin", &[Arg::Plain(reason)]);
                ess.send_mini(host, Sender::Console, &msg);
                let d = dc(ess);
                d.active = false;
                d.error = "discordErrorNoToken";
                d.queue.clear();
                return;
            }
            let d = dc(ess);
            d.self_id = json["id"].as_str().map(str::to_string);
            d.bot_name = json["global_name"].as_str().or(json["username"].as_str()).unwrap_or("").to_string();
        }
        Kind::Roles if ok => {
            let mut roles = HashMap::new();
            for r in json.as_array().into_iter().flatten() {
                let Some(id) = r["id"].as_str() else { continue };
                let perms = r["permissions"].as_str().and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
                roles.insert(id.to_string(), Role {
                    name: r["name"].as_str().unwrap_or("").to_string(),
                    color: r["color"].as_u64().unwrap_or(0) as u32,
                    position: r["position"].as_i64().unwrap_or(0),
                    admin: perms & 8 != 0,
                });
            }
            dc(ess).roles = roles;
        }
        Kind::Channel(id) if ok => {
            if let Some(name) = json["name"].as_str() {
                dc(ess).channel_names.insert(id, name.to_string());
            }
        }
        Kind::Webhooks(c) if ok => {
            let found = json.as_array().into_iter().flatten().find(|w| w["name"] == ADVANCED_RELAY_NAME && w["token"].is_string());
            match found {
                Some(w) => {
                    let hook = (str_of(&w["id"]), str_of(&w["token"]));
                    dc(ess).webhooks.insert(c, hook);
                }
                None => request(ess, "POST", format!("{API}/channels/{c}/webhooks"), Some(json!({"name": ADVANCED_RELAY_NAME})), Kind::CreateWebhook(c)),
            }
        }
        Kind::CreateWebhook(c) if ok => {
            if json["token"].is_string() {
                let hook = (str_of(&json["id"]), str_of(&json["token"]));
                dc(ess).webhooks.insert(c, hook);
            }
        }
        Kind::Poll(c) => {
            dc(ess).polling.remove(&c);
            if !ok {
                return;
            }
            let mut messages: Vec<Value> = json.as_array().cloned().unwrap_or_default();
            messages.sort_by_key(|m| m["id"].as_str().and_then(|i| i.parse::<u64>().ok()).unwrap_or(0));
            let primed = dc(ess).last_seen.get(&c).cloned().flatten().is_some();
            if let Some(newest) = messages.last().and_then(|m| m["id"].as_str()) {
                dc(ess).last_seen.insert(c.clone(), Some(newest.to_string()));
            } else if !primed {
                dc(ess).last_seen.insert(c.clone(), Some("0".into()));
            }
            if primed {
                for m in messages {
                    incoming(ess, host, &c, m);
                }
            }
        }
        Kind::Member(user) => {
            dc(ess).member_pending.remove(&user);
            let tick = dc(ess).tick;
            let member = if ok {
                Member {
                    nick: json["nick"].as_str().map(str::to_string),
                    roles: json["roles"].as_array().into_iter().flatten().filter_map(|r| r.as_str().map(str::to_string)).collect(),
                    fetched: tick,
                    missing: false,
                }
            } else {
                Member { fetched: tick, missing: true, ..Member::default() }
            };
            dc(ess).members.insert(user.clone(), member.clone());
            let waiting = dc(ess).waiting.remove(&user).unwrap_or_default();
            for (channel, m) in waiting {
                with_member(ess, host, &channel, m, &member);
            }
            let syncs = dc(ess).sync_waiting.remove(&user).unwrap_or_default();
            for (player, unsync) in syncs {
                apply_sync(ess, host, player, &user, &member, unsync);
            }
        }
        Kind::DmChannel(content) if ok => {
            if let Some(id) = json["id"].as_str() {
                request(ess, "POST", format!("{API}/channels/{id}/messages"), Some(json!({"content": content})), Kind::Send);
            }
        }
        Kind::Send if status == 403 => {
            let channel = req.url.split("/channels/").nth(1).and_then(|r| r.split('/').next()).unwrap_or("").to_string();
            let name = channel_name(ess, &channel);
            let msg = ess.tl("discordNoSendPermission", &[Arg::Plain(name)]);
            ess.send_mini(host, Sender::Console, &msg);
        }
        _ => {}
    }
}

// ===== Discord -> MC ==============================================================================

fn incoming(ess: &mut Ess, host: &mut dyn Host, channel: &str, m: Value) {
    let author = str_of(&m["author"]["id"]);
    if dc(ess).self_id.as_deref() == Some(author.as_str()) {
        return;
    }
    if let Some(hook) = m["webhook_id"].as_str() {
        let ours = dc(ess).webhooks.values().any(|(id, _)| id == hook);
        if !cfg_bool(ess, "chat.show-webhook-messages", false) || ours {
            return;
        }
    } else if m["author"]["bot"].as_bool() == Some(true) && !cfg_bool(ess, "chat.show-bot-messages", false) {
        return;
    }
    let content = m["content"].as_str().unwrap_or("").to_string();
    if console_channel(ess).as_deref() == Some(channel) {
        console_relay(ess, host, &content);
        return;
    }
    if command_of(ess, &content).is_none() {
        let filters = cfg_list(ess, "chat.discord-filter");
        if filters.iter().any(|f| Regex::new(f).find(&content)) {
            return;
        }
        if channel_keys(ess, channel).is_empty() {
            return;
        }
    }
    if let Some((name, args)) = command_of(ess, &content).filter(|(n, _)| matches!(n.as_str(), "link" | "unlink" | "account")) {
        let msg_id = str_of(&m["id"]);
        let username = m["author"]["username"].as_str().unwrap_or("").to_string();
        run_command(ess, host, channel, &msg_id, &author, &username, &Member::default(), &name, &args);
        return;
    }
    // Member data (nickname, roles) first: REST channel messages don't carry it.
    let guild = guild(ess);
    let tick = dc(ess).tick;
    if let Some(member) = dc(ess).members.get(&author).filter(|mb| tick.saturating_sub(mb.fetched) < MEMBER_TTL).cloned() {
        with_member(ess, host, channel, m, &member);
        return;
    }
    let d = dc(ess);
    d.waiting.entry(author.clone()).or_default().push((channel.to_string(), m));
    if d.member_pending.insert(author.clone()) {
        request(ess, "GET", format!("{API}/guilds/{guild}/members/{author}"), None, Kind::Member(author));
    }
}

/// The console channel's messages run as console commands (console.command-relay).
fn console_relay(ess: &mut Ess, host: &mut dyn Host, line: &str) {
    let line = line.trim().trim_start_matches('/');
    if line.is_empty() {
        return;
    }
    if ess.claims(line) {
        ess.dispatch(host, Sender::Console, line);
    } else {
        host.console_command(line);
    }
}

fn command_of(ess: &mut Ess, content: &str) -> Option<(String, Vec<String>)> {
    let prefix = cfg_str(ess, "message-command-prefix", "/");
    let rest = content.trim().strip_prefix(prefix.as_str())?;
    let mut words = rest.split_whitespace();
    let name = words.next()?.to_lowercase();
    ["execute", "msg", "list", "link", "unlink", "account"].contains(&name.as_str()).then(|| (name, words.map(str::to_string).collect()))
}

fn has_roles(ess: &mut Ess, user: &str, member: &Member, defs: &[String]) -> bool {
    let roles = dc(ess).roles.clone();
    if member.roles.iter().any(|r| roles.get(r).is_some_and(|role| role.admin)) {
        return true;
    }
    defs.iter().map(|d| d.trim()).any(|d| {
        d == "*" || d == user || member.roles.iter().any(|r| r == d || roles.get(r).is_some_and(|role| role.name.eq_ignore_ascii_case(d)))
    })
}

/// The member's roles, highest first, without the blacklisted ones (DiscordUtil.getHighestRole).
fn ranked_roles(ess: &mut Ess, member: &Member) -> Vec<(String, Role)> {
    let blacklist = cfg_list(ess, "discord-role-blacklist");
    let invert = cfg_bool(ess, "invert-discord-role-blacklist", false);
    let roles = dc(ess).roles.clone();
    let mut out: Vec<(String, Role)> = member.roles.iter().filter_map(|r| roles.get(r).map(|role| (r.clone(), role.clone()))).collect();
    out.sort_by_key(|(_, r)| -r.position);
    out.retain(|(id, r)| {
        let listed = blacklist.iter().any(|b| b == id || b == &r.name);
        listed == invert
    });
    out
}

fn with_member(ess: &mut Ess, host: &mut dyn Host, channel: &str, m: Value, member: &Member) {
    let author = str_of(&m["author"]["id"]);
    let content = m["content"].as_str().unwrap_or("").to_string();
    if let Some((name, args)) = command_of(ess, &content) {
        let msg_id = str_of(&m["id"]);
        let username = m["author"]["username"].as_str().unwrap_or("").to_string();
        run_command(ess, host, channel, &msg_id, &author, &username, member, &name, &args);
        return;
    }
    relay_to_mc(ess, host, channel, &m, member);
}

fn relay_to_mc(ess: &mut Ess, host: &mut dyn Host, channel: &str, m: &Value, member: &Member) {
    let keys = channel_keys(ess, channel);
    if keys.is_empty() {
        return;
    }
    let author = &m["author"];
    let username = author["username"].as_str().unwrap_or("").to_string();
    let discriminator = author["discriminator"].as_str().unwrap_or("0").to_string();
    let fullname = if discriminator == "0" || discriminator.is_empty() { username.clone() } else { format!("{username}#{discriminator}") };
    let nickname = member.nick.clone().or_else(|| author["global_name"].as_str().map(str::to_string)).unwrap_or_else(|| username.clone());
    // getContentDisplay: user mentions as @name.
    let mut text = m["content"].as_str().unwrap_or("").to_string();
    for mention in m["mentions"].as_array().into_iter().flatten() {
        let id = str_of(&mention["id"]);
        let name = format!("@{}", mention["username"].as_str().unwrap_or(""));
        text = text.replace(&format!("<@{id}>"), &name).replace(&format!("<@!{id}>"), &name);
    }
    if cfg_bool(ess, "show-discord-attachments", true) {
        for a in m["attachments"].as_array().into_iter().flatten() {
            if let Some(url) = a["url"].as_str() {
                text.push(' ');
                text.push_str(url);
            }
        }
    }
    let text = if cfg_bool(ess, "chat.filter-newlines", true) { text.replace('\n', " ") } else { text };
    let max = cfg(ess, "chat.discord-max-length").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;
    let stripped = abbreviate(text.trim(), max);
    let permitted = member.missing || {
        let defs = cfg_list(ess, "permit-formatting-roles");
        has_roles(ess, &str_of(&author["id"]), member, &defs)
    };
    let message = if permitted { replace_format(&stripped) } else { strip_legacy(&stripped, true) };
    if message.trim().is_empty() {
        return;
    }
    let ranked = ranked_roles(ess, member);
    let aliases = cfg(ess, "discord-roles-aliases").unwrap_or(Value::Null);
    let role = ranked.first().map(|(id, r)| aliases.get(id).or(aliases.get(&r.name)).map(str_of).map(|a| replace_format(&a)).unwrap_or_else(|| r.name.clone())).unwrap_or_default();
    let color = ranked.iter().find(|(_, r)| r.color != 0).map(|(_, r)| {
        let hex = format!("{:06x}", r.color);
        let mut code = String::from("§x");
        for c in hex.chars() {
            code.push('§');
            code.push(c);
        }
        code
    });
    let template = cfg(ess, "messages.discord-to-mc").filter(|v| !v.is_null()).map(|v| str_of(&v)).unwrap_or_else(|| "&6[#{channel}] &3{fullname}&7: &f{message}".into());
    let channel_display = channel_name(ess, channel);
    let line = fill(&replace_format(&template), &[
        ("channel", channel_display),
        ("username", username),
        ("discriminator", discriminator),
        ("fullname", fullname),
        ("nickname", nickname),
        ("color", color.unwrap_or_default()),
        ("message", message),
        ("role", role),
    ]);
    let mini = legacy_to_mini(&escape_tags(&line), false);
    let console = cfg_list(ess, "chat.relay-to-console");
    if keys.iter().any(|k| console.contains(k)) {
        ess.send_mini(host, Sender::Console, &mini);
    }
    let always_primary = cfg_bool(ess, "always-receive-primary", false);
    for p in host.online() {
        let name = host.name(p).unwrap_or_default();
        let receives = keys.iter().any(|k| {
            (always_primary && k.eq_ignore_ascii_case("primary")) || ess.perms.configured(p, &name, &format!("essentials.discord.receive.{k}")) == Some(true)
        });
        if receives {
            ess.send_mini(host, Sender::Player(p), &mini);
        }
    }
}

fn reply(ess: &mut Ess, channel: &str, msg_id: &str, content: &str) {
    let body = json!({"content": content, "allowed_mentions": {"parse": []}, "message_reference": {"message_id": msg_id, "fail_if_not_exists": false}});
    request(ess, "POST", format!("{API}/channels/{channel}/messages"), Some(body), Kind::Send);
}

#[allow(clippy::too_many_arguments)]
fn run_command(ess: &mut Ess, host: &mut dyn Host, channel: &str, msg_id: &str, author: &str, username: &str, member: &Member, name: &str, args: &[String]) {
    let essentials = matches!(name, "execute" | "msg" | "list");
    if essentials {
        if !cfg_bool(ess, &format!("commands.{name}.enabled"), true) {
            let text = tl_plain(ess, "discordErrorCommandDisabled", &[]);
            reply(ess, channel, msg_id, &text);
            return;
        }
        if let Some(Value::Array(_)) = cfg(ess, &format!("commands.{name}.allowed-roles")) {
            let defs = cfg_list(ess, &format!("commands.{name}.allowed-roles"));
            if !has_roles(ess, author, member, &defs) {
                let text = tl_plain(ess, "noAccessCommand", &[]);
                reply(ess, channel, msg_id, &text);
                return;
            }
        }
    }
    let hide = !essentials || cfg_bool(ess, &format!("commands.{name}.hide-command"), true);
    if hide {
        request(ess, "DELETE", format!("{API}/channels/{channel}/messages/{msg_id}"), None, Kind::Other);
    }
    let admin_defs = cfg_list(ess, &format!("commands.{name}.admin-roles"));
    let admin = has_roles(ess, author, member, &admin_defs);
    match name {
        "execute" => {
            let command = args.join(" ");
            if command.is_empty() {
                return;
            }
            let text = tl_plain(ess, "discordCommandExecuteReply", &[&command]);
            reply(ess, channel, msg_id, &text);
            let line = command.trim_start_matches('/').to_string();
            if ess.claims(&line) {
                ess.dispatch(host, Sender::Console, &line);
            } else {
                let (_, feedback) = host.console_command(&line);
                if !feedback.is_empty() {
                    reply(ess, channel, msg_id, &sanitize(&feedback.join("\n")));
                }
            }
        }
        "msg" => {
            if args.len() < 2 {
                return;
            }
            let found = ess.match_player(host, Sender::Console, &args[0], false).filter(|p| admin || !ess.is_vanished(*p));
            let Some(target) = found else {
                let err = ess.tl("playerNotFound", &[]);
                let text = plain(&ess.render(&ess.tl("errorWithMessage", &[Arg::Parsed(err)])));
                reply(ess, channel, msg_id, &text);
                return;
            };
            let target_name = player_name(ess, host, target);
            let user = ess.users.user(target).clone();
            if !admin && user.bool("ignore-msg") {
                let text = tl_plain(ess, "msgIgnore", &[&sanitize(&target_name)]);
                reply(ess, channel, msg_id, &text);
                return;
            }
            if user.bool("afk") {
                let text = match user.str("afk-message") {
                    Some(m) => tl_plain(ess, "userAFKWithMessage", &[&sanitize(&target_name), &sanitize(&m)]),
                    None => tl_plain(ess, "userAFK", &[&sanitize(&target_name)]),
                };
                reply(ess, channel, msg_id, &text);
            }
            let raw = args[1..].join(" ");
            let defs = cfg_list(ess, "permit-formatting-roles");
            let message = if has_roles(ess, author, member, &defs) { replace_format(&raw) } else { strip_legacy(&raw, true) };
            let me = ess.tl("meSender", &[]);
            let text = tl_plain(ess, "msgFormat", &[&me, &sanitize(&target_name), &sanitize(&strip_legacy(&message, false))]);
            reply(ess, channel, msg_id, &text);
            let recipient = ess.tl("meRecipient", &[]);
            ess.send_tl(host, Sender::Player(target), "msgFormat", &[Arg::Plain(username.to_string()), Arg::Parsed(recipient), Arg::Plain(message)]);
        }
        "list" => {
            let mut players: Vec<(String, String)> = Vec::new();
            for p in host.online() {
                if !admin && ess.is_vanished(p) {
                    continue;
                }
                let group = ess.group(host, p);
                players.push((group, player_name(ess, host, p)));
            }
            let max = host.max_players();
            let mut lines = vec![tl_plain(ess, "listAmount", &[&players.len().to_string(), &max.to_string()])];
            let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (g, n) in players {
                groups.entry(g).or_default().push(n);
            }
            let filter = args.first().map(|g| g.to_lowercase());
            for (g, mut names) in groups {
                if filter.as_ref().is_some_and(|f| *f != g) {
                    continue;
                }
                names.sort_by_key(|n| n.to_lowercase());
                lines.push(format!("{}{}", tl_plain(ess, "listGroupTag", &[&g]), names.join(", ")));
            }
            let text = sanitize(&lines.join("\n"));
            reply(ess, channel, msg_id, &text);
        }
        "link" => {
            let key = if uuid_of(ess, author).is_some() {
                "discordCommandLinkHasAccount"
            } else {
                let code = args.first().cloned().unwrap_or_default();
                match dc(ess).codes.remove(&code) {
                    None => "discordCommandLinkInvalidCode",
                    Some(player) => {
                        link_account(ess, host, player, author);
                        "discordCommandLinkLinked"
                    }
                }
            };
            let text = tl_plain(ess, key, &[]);
            reply(ess, channel, msg_id, &text);
        }
        "unlink" => {
            let key = match uuid_of(ess, author) {
                Some(player) => {
                    unlink_account(ess, host, player);
                    "discordCommandUnlinkUnlinked"
                }
                None => "discordCommandUnlinkInvalidCode",
            };
            let text = tl_plain(ess, key, &[]);
            reply(ess, channel, msg_id, &text);
        }
        "account" => {
            let target = args.first().map(|a| a.trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>').to_string()).filter(|a| !a.is_empty()).unwrap_or_else(|| author.to_string());
            let mention = format!("<@{target}>");
            let own = target == author;
            let text = match uuid_of(ess, &target) {
                None if own => tl_plain(ess, "discordCommandAccountResponseNotLinked", &[&mention]),
                None => tl_plain(ess, "discordCommandAccountResponseNotLinkedOther", &[&mention]),
                Some(player) => {
                    let name = player_name(ess, host, player);
                    if own {
                        tl_plain(ess, "discordCommandAccountResponseLinked", &[&name])
                    } else {
                        tl_plain(ess, "discordCommandAccountResponseLinkedOther", &[&mention, &name])
                    }
                }
            };
            reply(ess, channel, msg_id, &text);
        }
        _ => {}
    }
}

// ===== DiscordLink ================================================================================

fn link_policy(ess: &mut Ess) -> String {
    link_cfg(ess, "link-policy").map(|v| str_of(&v).to_lowercase()).unwrap_or_else(|| "none".into())
}

fn frozen(ess: &mut Ess, player: Uuid) -> bool {
    dc(ess).active && link_policy(ess) == "freeze" && linked_discord_id(ess, player).is_none()
}

fn prompt(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    let code = code_for(ess, host, player);
    let url = cfg_str(ess, "discord-url", "https://discord.gg/invite-code");
    ess.send_tl(host, Sender::Player(player), "discordLinkLoginPrompt", &[Arg::Plain(format!("/link {code}")), Arg::Plain(url)]);
}

/// The Discord account linked to a player.
pub fn linked_discord_id(ess: &mut Ess, player: Uuid) -> Option<String> {
    dc(ess).accounts.get(&uuid_string(player)).cloned()
}

fn uuid_of(ess: &mut Ess, discord: &str) -> Option<Uuid> {
    dc(ess).accounts.iter().find(|(_, id)| id.as_str() == discord).and_then(|(u, _)| parse_uuid(u))
}

fn save_accounts(ess: &mut Ess) {
    let text = serde_json::to_string(&dc(ess).accounts).unwrap_or_else(|_| "{}".into());
    let path = ess.dir.join("discordlink/accounts.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, text);
}

/// AccountLinkManager.createCode: an existing pending code, or a new 8-character one.
fn code_for(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    if let Some((code, _)) = dc(ess).codes.iter().find(|(_, u)| **u == player) {
        return code.clone();
    }
    let now = host.now_millis() as u64;
    let d = dc(ess);
    if d.rng == 0 {
        d.rng = (now ^ (player as u64) ^ ((player >> 64) as u64)) | 1;
    }
    loop {
        let mut code = String::new();
        for _ in 0..8 {
            d.rng ^= d.rng << 13;
            d.rng ^= d.rng >> 7;
            d.rng ^= d.rng << 17;
            code.push(b"abcdefghijklmnopqrstuvwxyz0123456789"[(d.rng % 36) as usize] as char);
        }
        if !d.codes.contains_key(&code) {
            d.codes.insert(code.clone(), player);
            return code;
        }
    }
}

/// Links a player to a Discord account (AccountLinkManager.registerAccount): stored, then roles synced.
pub fn link_account(ess: &mut Ess, _host: &mut dyn Host, player: Uuid, discord: &str) {
    let d = dc(ess);
    d.accounts.retain(|_, id| id != discord);
    d.accounts.insert(uuid_string(player), discord.to_string());
    d.codes.retain(|_, u| *u != player);
    save_accounts(ess);
    sync_roles(ess, player, discord, false);
}

/// Unlinks a player; false when they had no link. Kicks or freezes them again per link-policy.
pub fn unlink_account(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> bool {
    let Some(id) = dc(ess).accounts.remove(&uuid_string(player)) else { return false };
    save_accounts(ess);
    unsync(ess, host, player, &id);
    if host.online().contains(&player) {
        match link_policy(ess).as_str() {
            "kick" => {
                let code = code_for(ess, host, player);
                let url = cfg_str(ess, "discord-url", "https://discord.gg/invite-code");
                let text = ess.tl("discordLinkLoginKick", &[Arg::Plain(format!("/link {code}")), Arg::Plain(url)]);
                let spans = ess.render(&text.replace('\n', "<newline>"));
                host.kick(player, &spans);
            }
            "freeze" => prompt(ess, host, player),
            _ => {}
        }
    }
    true
}

fn is_example_role(role: &str) -> bool {
    matches!(role, "0" | "11111111111111111" | "22222222222222222" | "33333333333333333")
}

fn role_id_str(v: &Value) -> String {
    let s = str_of(v);
    if !s.is_empty() && s.chars().all(|c| c == '0') { "0".into() } else { s }
}

/// role-sync.groups (group -> role) and role-sync.roles (role -> group), example entries and unknown groups skipped.
fn sync_maps(ess: &mut Ess) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let groups: Vec<String> = ess.perms.groups.keys().cloned().collect();
    let known = dc(ess).roles.clone();
    let role_ok = |r: &str| known.is_empty() || known.contains_key(r);
    let mut g2r = Vec::new();
    if let Some(Value::Object(m)) = link_cfg(ess, "role-sync.groups") {
        for (g, r) in m {
            let r = role_id_str(&r);
            if !is_example_role(&r) && groups.contains(&g.to_lowercase()) && role_ok(&r) {
                g2r.push((g.to_lowercase(), r));
            }
        }
    }
    let mut r2g = Vec::new();
    if let Some(Value::Object(m)) = link_cfg(ess, "role-sync.roles") {
        for (r, g) in m {
            let g = str_of(&g).to_lowercase();
            if !is_example_role(&r) && groups.contains(&g) && role_ok(&r) {
                r2g.push((r, g));
            }
        }
    }
    (g2r, r2g)
}

fn sync_roles(ess: &mut Ess, player: Uuid, discord: &str, unsync: bool) {
    if !dc(ess).active {
        return;
    }
    let (g2r, r2g) = sync_maps(ess);
    if g2r.is_empty() && r2g.is_empty() {
        return;
    }
    let guild = guild(ess);
    let d = dc(ess);
    d.sync_waiting.entry(discord.to_string()).or_default().push((player, unsync));
    if d.member_pending.insert(discord.to_string()) {
        request(ess, "GET", format!("{API}/guilds/{guild}/members/{discord}"), None, Kind::Member(discord.to_string()));
    }
}

/// RoleSyncManager.unSync: mapped groups removed now, mapped roles removed once the member is known.
fn unsync(ess: &mut Ess, host: &mut dyn Host, player: Uuid, discord: &str) {
    let (_, r2g) = sync_maps(ess);
    if link_bool(ess, "role-sync.remove-groups", true) {
        for (_, g) in &r2g {
            set_group(ess, host, player, g, false);
        }
    }
    if link_bool(ess, "role-sync.remove-roles", true) {
        sync_roles(ess, player, discord, true);
    }
}

fn apply_sync(ess: &mut Ess, host: &mut dyn Host, player: Uuid, discord: &str, member: &Member, unsync_only: bool) {
    let (g2r, r2g) = sync_maps(ess);
    let guild = guild(ess);
    let role_url = |r: &str| format!("{API}/guilds/{guild}/members/{discord}/roles/{r}");
    if unsync_only {
        for (_, r) in &g2r {
            if member.roles.contains(r) {
                request(ess, "DELETE", role_url(r), None, Kind::Other);
            }
        }
        return;
    }
    if member.missing {
        if link_bool(ess, "unlink-on-leave", true) {
            dc(ess).accounts.remove(&uuid_string(player));
            save_accounts(ess);
        }
        for (_, g) in &r2g {
            if link_bool(ess, "role-sync.remove-groups", true) {
                set_group(ess, host, player, g, false);
            }
        }
        return;
    }
    let name = player_name(ess, host, player);
    let groups = if link_bool(ess, "role-sync.primary-group-only", false) {
        vec![ess.group(host, player)]
    } else {
        ess.perms.groups_of(player, &name)
    };
    let remove_roles = link_bool(ess, "role-sync.remove-roles", true);
    let remove_groups = link_bool(ess, "role-sync.remove-groups", true);
    for (g, r) in &g2r {
        let has = member.roles.contains(r);
        if groups.contains(g) && !has {
            request(ess, "PUT", role_url(r), None, Kind::Other);
        } else if remove_roles && !groups.contains(g) && has {
            request(ess, "DELETE", role_url(r), None, Kind::Other);
        }
    }
    for (r, g) in &r2g {
        let has = member.roles.contains(r);
        if has && !groups.contains(g) {
            set_group(ess, host, player, g, true);
        } else if remove_groups && !has && groups.contains(g) {
            set_group(ess, host, player, g, false);
        }
    }
}

/// Adds or removes a group in permissions.yml (`users.<name or uuid>.groups`) and reloads permissions.
fn set_group(ess: &mut Ess, host: &mut dyn Host, player: Uuid, group: &str, add: bool) {
    let name = player_name(ess, host, player);
    let mut current = ess.perms.groups_of(player, &name);
    let had = current.iter().any(|g| g == group);
    if had == add {
        return;
    }
    if add {
        current.push(group.to_string());
    } else {
        current.retain(|g| g != group);
    }
    let mut yaml = ess.read_yaml("permissions.yml");
    if !yaml.is_object() {
        yaml = json!({});
    }
    if !yaml["users"].is_object() {
        yaml["users"] = json!({});
    }
    let users = yaml["users"].as_object_mut().expect("users map");
    let uuid = uuid_string(player);
    let key = users.keys().find(|k| k.eq_ignore_ascii_case(&uuid) || k.eq_ignore_ascii_case(&name)).cloned().unwrap_or(uuid);
    let entry = users.entry(key).or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    entry["groups"] = json!(current);
    ess.write_yaml("permissions.yml", &yaml);
    ess.perms = Perms::load(&yaml);
}

fn resync_batch(ess: &mut Ess) {
    let entries: Vec<(String, String)> = dc(ess).accounts.iter().map(|(u, d)| (u.clone(), d.clone())).collect();
    if entries.is_empty() {
        dc(ess).sync_cursor = 0;
        return;
    }
    let start = if dc(ess).sync_cursor >= entries.len() { 0 } else { dc(ess).sync_cursor };
    let end = (start + 50).min(entries.len());
    dc(ess).sync_cursor = if end >= entries.len() { 0 } else { end };
    for (u, id) in &entries[start..end] {
        if let Some(player) = parse_uuid(u) {
            sync_roles(ess, player, id, false);
        }
    }
}

fn on_moved(ess: &mut Ess, host: &mut dyn Host, player: Uuid, from: &Loc, _to: &Loc) {
    if frozen(ess, player) {
        host.teleport(player, from);
        ess.last_loc.insert(player, from.clone());
    }
}

fn on_command(ess: &mut Ess, host: &mut dyn Host, sender: Sender, _line: &str) -> Allow {
    match sender {
        Sender::Player(p) if frozen(ess, p) => {
            prompt(ess, host, p);
            Allow::No
        }
        _ => Allow::Yes,
    }
}

fn on_interact(ess: &mut Ess, _host: &mut dyn Host, player: Uuid, _block: Option<(&str, [i32; 3], &str)>, _held: Option<&Item>) -> Allow {
    if frozen(ess, player) { Allow::No } else { Allow::Yes }
}

// ===== Commands ===================================================================================

fn cmd_discord(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let url = cfg_str(ctx.ess, "discord-url", "https://discord.gg/invite-code");
    ctx.tl("discordCommandLink", &[Arg::Plain(url)]);
    Ok(())
}

fn cmd_discordbroadcast(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    if !ctx.has(&format!("essentials.discordbroadcast.{}", args[0])) {
        return Err(CmdError::Tl("discordbroadcastPermission", vec![Arg::Plain(args[0].clone())]));
    }
    let mut message = final_arg(args, 1);
    if !ctx.has("essentials.discordbroadcast.markdown") {
        message = sanitize(&message);
    }
    if !dc(ctx.ess).active {
        let key = dc(ctx.ess).error;
        return Err(CmdError::Tl(if key.is_empty() { "discordErrorNoToken" } else { key }, Vec::new()));
    }
    let Some(channel) = channel_id(ctx.ess, &args[0]) else {
        return Err(CmdError::Tl("discordbroadcastInvalidChannel", vec![Arg::Plain(args[0].clone())]));
    };
    let mut body = json!({"content": message});
    if !ctx.has("essentials.discordbroadcast.ping") {
        body["allowed_mentions"] = json!({"parse": ["users"]});
    }
    request(ctx.ess, "POST", format!("{API}/channels/{channel}/messages"), Some(body), Kind::Send);
    let name = channel_name(ctx.ess, &channel);
    ctx.tl("discordbroadcastSent", &[Arg::Plain(format!("#{name}"))]);
    Ok(())
}

fn cmd_link(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let player = ctx.player()?;
    if !dc(ctx.ess).active {
        let key = dc(ctx.ess).error;
        return Err(CmdError::Tl(if key.is_empty() { "discordErrorNoToken" } else { key }, Vec::new()));
    }
    if linked_discord_id(ctx.ess, player).is_some() {
        ctx.tl("discordLinkLinkedAlready", &[]);
        return Ok(());
    }
    let pending = dc(ctx.ess).codes.iter().find(|(_, u)| **u == player).map(|(c, _)| c.clone());
    match pending {
        Some(code) => ctx.tl("discordLinkPending", &[Arg::Plain(format!("/link {code}"))]),
        None => {
            let code = code_for(ctx.ess, ctx.host, player);
            ctx.tl("discordLinkLinked", &[Arg::Plain(format!("/link {code}"))]);
        }
    }
    Ok(())
}

fn cmd_unlink(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let player = ctx.player()?;
    if unlink_account(ctx.ess, ctx.host, player) {
        ctx.tl("discordLinkUnlinked", &[]);
    } else {
        ctx.tl("discordLinkNoAccount", &[]);
    }
    Ok(())
}

fn tab_discordbroadcast(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    if args.len() != 1 {
        return Vec::new();
    }
    let names: Vec<String> = channel_map(ctx.ess).into_iter().map(|(k, _)| k).collect();
    names.into_iter().filter(|n| ctx.has(&format!("essentials.discordbroadcast.{n}"))).collect()
}

pub fn register(ess: &mut Ess) {
    ess.register(Command { name: "discord", run: cmd_discord, tab: None, usages: &[("/<command>", "discordCommandUsage1Description")] });
    ess.register(Command {
        name: "discordbroadcast",
        run: cmd_discordbroadcast,
        tab: Some(tab_discordbroadcast),
        usages: &[("/<command> <channel> <msg>", "discordbroadcastCommandUsage1Description")],
    });
    ess.register(Command { name: "link", run: cmd_link, tab: None, usages: &[("/<command>", "linkCommandUsage1Description")] });
    ess.register(Command { name: "unlink", run: cmd_unlink, tab: None, usages: &[("/<command>", "unlinkCommandUsage1Description")] });
    // First in line: it only observes (Pass) unless DiscordLink freezes the player, and later chat
    // listeners that replace the message would otherwise stop it from seeing chat at all.
    ess.listeners.chat.insert(0, on_chat);
    ess.listeners.join.push(on_join);
    ess.listeners.quit.push(on_quit);
    ess.listeners.death.push(on_death);
    ess.listeners.tick.push(on_tick);
    ess.listeners.moved.push(on_moved);
    ess.listeners.command.push(on_command);
    ess.listeners.interact.push(on_interact);
}

// ===== A small regex (chat.discord-filter, console-filter) =======================================
// Java's Pattern isn't available: this covers literals, `.`, classes `[...]`/`[^...]`, `\d \w \s`, anchors
// `^ $`, the quantifiers `* + ?` and top-level `|`. Groups are matched literally.

#[derive(Clone, Debug)]
enum Atom {
    Any,
    Char(char),
    Class(Vec<(char, char)>, bool),
    Start,
    End,
}

#[derive(Clone, Debug)]
struct Node {
    atom: Atom,
    min: usize,
    max: usize,
}

pub struct Regex {
    alts: Vec<Vec<Node>>,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        let chars: Vec<char> = pattern.chars().collect();
        let mut alts = vec![Vec::new()];
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            i += 1;
            let atom = match c {
                '|' => {
                    alts.push(Vec::new());
                    continue;
                }
                '^' => Atom::Start,
                '$' => Atom::End,
                '.' => Atom::Any,
                '\\' if i < chars.len() => {
                    let e = chars[i];
                    i += 1;
                    escape_atom(e)
                }
                '[' => {
                    let negate = chars.get(i) == Some(&'^');
                    if negate {
                        i += 1;
                    }
                    let mut ranges = Vec::new();
                    let mut first = true;
                    while i < chars.len() && (chars[i] != ']' || first) {
                        first = false;
                        let mut lo = chars[i];
                        i += 1;
                        if lo == '\\' && i < chars.len() {
                            if let Atom::Class(r, _) = escape_atom(chars[i]) {
                                ranges.extend(r);
                                i += 1;
                                continue;
                            }
                            lo = chars[i];
                            i += 1;
                        }
                        if chars.get(i) == Some(&'-') && chars.get(i + 1).is_some_and(|c| *c != ']') {
                            ranges.push((lo, chars[i + 1]));
                            i += 2;
                        } else {
                            ranges.push((lo, lo));
                        }
                    }
                    i += 1;
                    Atom::Class(ranges, negate)
                }
                other => Atom::Char(other),
            };
            let (min, max) = match chars.get(i) {
                Some('*') => (0, usize::MAX),
                Some('+') => (1, usize::MAX),
                Some('?') => (0, 1),
                _ => (1, 1),
            };
            if (min, max) != (1, 1) {
                i += 1;
            }
            alts.last_mut().expect("an alternative").push(Node { atom, min, max });
        }
        Regex { alts }
    }

    /// Matcher.find: anywhere in the text.
    pub fn find(&self, text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        self.alts.iter().any(|nodes| (0..=chars.len()).any(|start| matches_at(nodes, &chars, start)))
    }
}

fn escape_atom(e: char) -> Atom {
    match e {
        'd' => Atom::Class(vec![('0', '9')], false),
        'D' => Atom::Class(vec![('0', '9')], true),
        'w' => Atom::Class(vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')], false),
        'W' => Atom::Class(vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')], true),
        's' => Atom::Class(vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')], false),
        'S' => Atom::Class(vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')], true),
        other => Atom::Char(other),
    }
}

fn atom_matches(atom: &Atom, c: char) -> bool {
    match atom {
        Atom::Any => c != '\n',
        Atom::Char(x) => *x == c,
        Atom::Class(ranges, negate) => ranges.iter().any(|(lo, hi)| *lo <= c && c <= *hi) != *negate,
        Atom::Start | Atom::End => false,
    }
}

fn matches_at(nodes: &[Node], chars: &[char], pos: usize) -> bool {
    let Some(node) = nodes.first() else { return true };
    match node.atom {
        Atom::Start => return pos == 0 && matches_at(&nodes[1..], chars, pos),
        Atom::End => return pos == chars.len() && matches_at(&nodes[1..], chars, pos),
        _ => {}
    }
    let mut count = 0;
    while count < node.max && pos + count < chars.len() && atom_matches(&node.atom, chars[pos + count]) {
        count += 1;
    }
    if count < node.min {
        return false;
    }
    (node.min..=count).rev().any(|k| matches_at(&nodes[1..], chars, pos + k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ess_testkit::*;

    /// Joins and drops the join-time lines other areas send (motd, mail notices...).
    fn join(t: &mut TestServer, name: &str) -> Uuid {
        let u = t.join(name);
        t.tick(1);
        t.said(u);
        u
    }
    use crate::game::{ess_defaults, ess_yaml};
    use serde_json::{Value, json};

    const CFG: &str = "token: tok\nguild: 1\ndiscord-url: https://discord.gg/abc\nchannels:\n  primary: 100\n  staff: 200\n";

    fn write_merged(t: &TestServer, name: &str, layers: &[&str]) {
        let mut base = ess_yaml::parse(ess_defaults::file(name).unwrap()).unwrap();
        for layer in layers {
            if !layer.is_empty() {
                merge(&mut base, ess_yaml::parse(layer).unwrap());
            }
        }
        let path = t.ess.dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, ess_yaml::to_string(&base)).unwrap();
    }

    /// A server whose Discord module has a token (plus `extra` discord config and `link` discordlink config).
    fn server_with(extra: &str, link: &str) -> TestServer {
        let mut t = TestServer::new();
        write_merged(&t, "discord/config.yml", &[CFG, extra]);
        write_merged(&t, "discordlink/config.yml", &[link]);
        reload(&mut t.ess);
        t
    }

    fn server() -> TestServer {
        server_with("", "")
    }

    /// Drains the HTTP requests: (id, method, url, body as JSON or Null).
    fn take(t: &mut TestServer) -> Vec<(u64, String, String, Value)> {
        std::mem::take(&mut t.host.http_requests)
            .into_iter()
            .map(|(id, m, url, _, body)| (id, m, url, body.and_then(|b| serde_json::from_str(&b).ok()).unwrap_or(Value::Null)))
            .collect()
    }

    /// The host's answer to a request, handed to the module as the world area's HTTP dispatcher does.
    fn answer(t: &mut TestServer, id: u64, status: u16, body: Value) {
        assert!(on_http(&mut t.ess, &mut t.host, id, status, &body.to_string()));
    }

    fn posts_to(reqs: &[(u64, String, String, Value)], url_end: &str) -> Vec<Value> {
        reqs.iter().filter(|r| r.1 == "POST" && r.2.ends_with(url_end)).map(|r| r.3.clone()).collect()
    }

    /// First tick: answers the startup requests (bot user, guild roles, channel names).
    fn boot(t: &mut TestServer) -> Vec<(u64, String, String, Value)> {
        t.tick(1);
        let reqs = take(t);
        for (id, m, url, _) in &reqs {
            let body = if url.ends_with("/users/@me") {
                json!({"id": "999", "username": "EssBot"})
            } else if url.ends_with("/guilds/1/roles") {
                json!([
                    {"id": "50", "name": "Admins", "color": 16711680, "position": 3, "permissions": "8"},
                    {"id": "51", "name": "Color Codes", "color": 0, "position": 2, "permissions": "0"},
                    {"id": "52", "name": "Members", "color": 255, "position": 1, "permissions": "0"},
                    {"id": "777", "name": "AdminRole", "color": 0, "position": 4, "permissions": "0"},
                    {"id": "888", "name": "Vip", "color": 0, "position": 0, "permissions": "0"}
                ])
            } else if url.ends_with("/channels/100") {
                json!({"id": "100", "name": "general"})
            } else if url.ends_with("/channels/200") {
                json!({"id": "200", "name": "staff-chat"})
            } else if url.ends_with("/webhooks") && m == "GET" {
                json!([])
            } else {
                json!({})
            };
            answer(t, *id, 200, body);
        }
        t.tick(1);
        reqs
    }

    fn msg(id: u64, author: &str, username: &str, content: &str) -> Value {
        json!({"id": id.to_string(), "channel_id": "100", "content": content,
               "author": {"id": author, "username": username, "discriminator": "0"}, "attachments": []})
    }

    /// Ticks until the module polls `channel`, answers with `messages`; returns the poll's URL.
    fn poll(t: &mut TestServer, channel: &str, messages: Value) -> String {
        for _ in 0..400 {
            t.tick(1);
            let reqs = take(t);
            let mut found = None;
            for (id, m, url, _) in &reqs {
                if m == "GET" && url.contains(&format!("/channels/{channel}/messages")) && found.is_none() {
                    found = Some((*id, url.clone()));
                } else {
                    answer(t, *id, 200, json!({}));
                }
            }
            if let Some((id, url)) = found {
                answer(t, id, 200, messages);
                t.tick(1);
                return url;
            }
        }
        panic!("no poll of channel {channel}");
    }

    /// Primes polling (the first poll only records the newest message id).
    fn prime(t: &mut TestServer) {
        let url = poll(t, "100", json!([{"id": "10", "content": "old", "author": {"id": "1", "username": "x"}}]));
        assert!(url.contains("limit=1"), "{url}");
    }

    /// Answers a pending member lookup for `user` with `roles`/`nick`.
    fn member(t: &mut TestServer, user: &str, roles: &[&str], nick: Option<&str>) {
        t.tick(1);
        let reqs = take(t);
        let mut seen = false;
        for (id, m, url, _) in &reqs {
            if m == "GET" && url.ends_with(&format!("/guilds/1/members/{user}")) {
                answer(t, *id, 200, json!({"user": {"id": user}, "nick": nick, "roles": roles}));
                seen = true;
            } else {
                answer(t, *id, 200, json!({}));
            }
        }
        assert!(seen, "no member lookup for {user} in {reqs:?}");
        t.tick(1);
    }

    fn replies(reqs: &[(u64, String, String, Value)]) -> Vec<String> {
        posts_to(reqs, "/channels/100/messages").iter().filter_map(|b| b["content"].as_str().map(str::to_string)).collect()
    }

    fn accounts(t: &TestServer) -> Value {
        serde_json::from_str(&std::fs::read_to_string(t.ess.dir.join("discordlink/accounts.json")).unwrap_or_default()).unwrap_or(json!({}))
    }

    // ----- /discord -----------------------------------------------------------------------------------

    #[test]
    fn discord_shows_the_invite_link() {
        let mut t = server();
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/discord"), ["Join our Discord server at https://discord.gg/abc!"]);
        assert_eq!(t.console("ediscord"), ["Join our Discord server at https://discord.gg/abc!"]);
        let bob = join(&mut t, "Bob");
        assert_eq!(t.cmd(bob, "/discord"), ["You do not have access to that command."]);
    }

    #[test]
    fn discord_works_without_a_token() {
        let mut t = TestServer::new();
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/discord"), ["Join our Discord server at https://discord.gg/invite-code!"]);
        t.tick(300);
        assert!(t.host.http_requests.is_empty(), "inactive module made requests");
        assert_ne!(t.chat(alice, "hi"), Chat::Cancel);
    }

    // ----- /discordbroadcast -----------------------------------------------------------------------

    #[test]
    fn discordbroadcast_usage_permission_and_channels() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        let usage = t.cmd(alice, "/dbc staff");
        assert_eq!(usage[0], "Command Help: /dbc");
        assert_eq!(t.cmd(alice, "/dbc nochan hello"), ["Error: Discord channel nochan does not exist."]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.discordbroadcast, essentials.discordbroadcast.primary]\n");
        let bob = join(&mut t, "Bob");
        assert_eq!(t.cmd(bob, "/dbc staff hi"), ["Error: You do not have permission to send messages to the staff channel."]);
        t.tick(1);
        take(&mut t);
        assert_eq!(t.cmd(bob, "/dbc primary Hello *world* @everyone"), ["Message sent to #general!"]);
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["content"], "Hello \\*world\\* @everyone");
        assert_eq!(sent[0]["allowed_mentions"], json!({"parse": ["users"]}));
    }

    #[test]
    fn discordbroadcast_markdown_and_ping_for_ops() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        t.tick(1);
        take(&mut t);
        assert_eq!(t.cmd(alice, "/discordbroadcast staff **bold**"), ["Message sent to #staff-chat!"]);
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/200/messages");
        assert_eq!(sent[0]["content"], "**bold**");
        assert!(sent[0].get("allowed_mentions").is_none());
        // A raw channel id works too.
        assert_eq!(t.cmd(alice, "/dbc 200 hi"), ["Message sent to #staff-chat!"]);
    }

    #[test]
    fn discordbroadcast_without_a_token() {
        let mut t = TestServer::new();
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/dbc primary hi"), ["Error: No token provided! Please follow the tutorial in the config in order to setup the plugin."]);
    }

    // ----- MC -> Discord ----------------------------------------------------------------------------

    #[test]
    fn chat_goes_to_the_primary_channel() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        t.tick(1);
        take(&mut t);
        assert_ne!(t.chat(alice, "hello *there*"), Chat::Cancel);
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["content"], "Alice: hello *there*");
        let bob = join(&mut t, "Bob_1");
        t.tick(1);
        take(&mut t);
        t.chat(bob, "a_b @here");
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent[0]["content"], "Bob\\_1: a\\_b @here");
        assert_eq!(sent[0]["allowed_mentions"], json!({"parse": ["users"]}));
    }

    #[test]
    fn message_types_route_or_disable() {
        let mut t = server_with("message-types:\n  chat: staff\n  join: none\n  first-join: none\n", "");
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        assert!(posts_to(&take(&mut t), "/messages").is_empty(), "join is disabled");
        t.chat(alice, "hi");
        t.tick(1);
        let reqs = take(&mut t);
        assert_eq!(posts_to(&reqs, "/channels/200/messages")[0]["content"], "Alice: hi");
    }

    #[test]
    fn join_first_join_and_quit_messages() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent[0]["content"], ":arrow_right: :first_place: Alice has joined the server for the first time!");
        t.quit(alice);
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent[0]["content"], ":arrow_left: Alice has left!");
        join(&mut t, "Alice");
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent[0]["content"], ":arrow_right: Alice has joined!");
    }

    #[test]
    fn join_formats_take_online_and_unique() {
        let mut t = server_with("messages:\n  join: '{username} joined ({online} online, {unique} unique)'\n", "");
        boot(&mut t);
        join(&mut t, "Alice");
        let bob = join(&mut t, "Bob");
        t.quit(bob);
        t.tick(1);
        take(&mut t);
        join(&mut t, "Bob");
        t.tick(1);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent[0]["content"], "Bob joined (2 online, 2 unique)");
    }

    #[test]
    fn silent_join_and_vanished_quit_are_hidden() {
        let mut t = server_with("", "");
        let config = format!("{}allow-silent-join-quit: true\n", crate::game::ess_testkit::TEST_BASELINE);
        std::fs::write(t.ess.dir.join("config.yml"), config).unwrap();
        t.ess.reload();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.silentjoin]\n");
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        assert!(posts_to(&take(&mut t), "/messages").is_empty());
        t.ess.users.user(alice).set("vanished", json!(true));
        t.quit(alice);
        t.tick(1);
        let posts = posts_to(&take(&mut t), "/messages");
        assert!(posts.is_empty(), "{posts:?}");
    }

    #[test]
    fn deaths_are_relayed() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        take(&mut t);
        let at = t.loc(alice);
        t.ess.on_death(&mut t.host, alice, &at);
        t.tick(1);
        assert_eq!(posts_to(&take(&mut t), "/channels/100/messages")[0]["content"], ":skull: Alice died");
    }

    #[test]
    fn server_start_message_on_first_tick() {
        let mut t = server();
        let reqs = boot(&mut t);
        let sent = posts_to(&reqs, "/channels/100/messages");
        assert_eq!(sent.len(), 1);
        let content = sent[0]["content"].as_str().unwrap();
        assert!(content.starts_with(":white_check_mark: The server has started in ") && content.ends_with(" seconds!"), "{content}");
    }

    #[test]
    fn api_relays_for_other_areas() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        take(&mut t);
        relay_afk(&mut t.ess, &mut t.host, alice, true);
        relay_afk(&mut t.ess, &mut t.host, alice, false);
        relay_action(&mut t.ess, &mut t.host, alice, "waves");
        relay_kick(&mut t.ess, &mut t.host, alice, "spam");
        relay_advancement(&mut t.ess, &mut t.host, alice, "Stone Age");
        t.tick(1);
        let reqs = take(&mut t);
        let primary: Vec<_> = posts_to(&reqs, "/channels/100/messages").iter().map(|b| b["content"].as_str().unwrap().to_string()).collect();
        assert_eq!(primary, [
            ":person_walking: Alice is now AFK!",
            ":keyboard: Alice is no longer AFK!",
            ":person_biking: Alice *waves*",
            ":medal: Alice has completed the advancement **Stone Age**!",
        ]);
        assert_eq!(posts_to(&reqs, "/channels/200/messages")[0]["content"], "Alice was kicked with reason: spam");
        // private-chat is `none` by default.
        relay_private(&mut t.ess, &mut t.host, alice, "Bob", "hi");
        t.tick(1);
        assert!(posts_to(&take(&mut t), "/messages").is_empty());
    }

    #[test]
    fn server_stop_message() {
        let mut t = server();
        boot(&mut t);
        on_server_stop(&mut t.ess, &mut t.host);
        let sent = posts_to(&take(&mut t), "/channels/100/messages");
        assert_eq!(sent[0]["content"], ":octagonal_sign: The server has stopped!");
    }

    #[test]
    fn rate_limits_are_respected() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        take(&mut t);
        t.chat(alice, "one");
        t.tick(1);
        let reqs = take(&mut t);
        let (id, _, url, _) = reqs.iter().find(|r| r.1 == "POST").unwrap().clone();
        answer(&mut t, id, 429, json!({"retry_after": 1.5, "global": false}));
        t.tick(1);
        t.tick(25);
        assert!(posts_to(&take(&mut t), "/messages").is_empty(), "retried too early");
        t.tick(10);
        let again = take(&mut t);
        let retried: Vec<_> = again.iter().filter(|r| r.1 == "POST" && r.2 == url).collect();
        assert_eq!(retried.len(), 1);
        assert_eq!(retried[0].3["content"], "Alice: one");
    }

    #[test]
    fn webhooks_carry_the_players_name_and_avatar() {
        let mut t = server_with("show-avatar: true\nmessages:\n  mc-to-discord-name-format: '{displayname}'\n", "");
        let reqs = boot(&mut t);
        assert!(reqs.iter().any(|r| r.1 == "GET" && r.2.ends_with("/channels/100/webhooks")));
        let reqs = take(&mut t);
        let create = reqs.iter().find(|r| r.1 == "POST" && r.2.ends_with("/channels/100/webhooks")).expect("webhook created");
        assert_eq!(create.3["name"], "EssX Advanced Relay");
        answer(&mut t, create.0, 200, json!({"id": "9", "token": "wtok", "name": "EssX Advanced Relay"}));
        t.tick(1);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        take(&mut t);
        t.chat(alice, "hi");
        t.tick(1);
        let reqs = take(&mut t);
        let hook = posts_to(&reqs, "/webhooks/9/wtok");
        assert_eq!(hook.len(), 1, "{reqs:?}");
        assert_eq!(hook[0]["content"], "Alice: hi");
        assert_eq!(hook[0]["username"], "Alice");
        assert_eq!(hook[0]["avatar_url"], format!("https://crafthead.net/helm/{}", uuid_string(alice)));
    }

    // ----- Discord -> MC ----------------------------------------------------------------------------

    #[test]
    fn discord_chat_reaches_players_with_the_receive_permission() {
        let mut t = server();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: []\nusers:\n  alice:\n    permissions: [essentials.discord.receive.primary]\n");
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        let bob = join(&mut t, "Bob");
        prime(&mut t);
        let url = poll(&mut t, "100", json!([msg(12, "7", "steve", "second"), msg(11, "7", "steve", "first")]));
        assert!(url.contains("after=10"), "{url}");
        member(&mut t, "7", &[], Some("Stevie"));
        assert_eq!(t.said(alice), ["[#general] steve: first", "[#general] steve: second"]);
        assert!(t.said(bob).is_empty());
        let url = poll(&mut t, "100", json!([]));
        assert!(url.contains("after=12"), "{url}");
    }

    #[test]
    fn discord_to_mc_format_and_always_receive_primary() {
        let mut t = server_with("always-receive-primary: true\nmessages:\n  discord-to-mc: '[{channel}] {nickname} ({role}): {message}'\nchat:\n  relay-to-console: [primary]\n", "");
        boot(&mut t);
        let bob = join(&mut t, "Bob");
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "7", "steve", "line1\nline2")]));
        member(&mut t, "7", &["51"], Some("Stevie"));
        assert_eq!(t.said(bob), ["[general] Stevie (Color Codes): line1 line2"]);
        assert_eq!(t.host.said(Sender::Console), ["[general] Stevie (Color Codes): line1 line2"]);
    }

    #[test]
    fn discord_filter_bots_and_own_messages_are_skipped() {
        let mut t = server_with("always-receive-primary: true\n", "");
        boot(&mut t);
        let bob = join(&mut t, "Bob");
        prime(&mut t);
        let mut bot = msg(13, "8", "otherbot", "beep");
        bot["author"]["bot"] = json!(true);
        let mut own = msg(14, "999", "EssBot", "Alice: hi");
        own["author"]["bot"] = json!(true);
        let mut hook = msg(15, "9", "hook", "from a webhook");
        hook["webhook_id"] = json!("9");
        poll(&mut t, "100", json!([msg(11, "7", "steve", "!command"), msg(12, "7", "steve", "?question"), bot, own, hook]));
        t.tick(2);
        assert!(t.said(bob).is_empty());
        assert!(take(&mut t).iter().all(|r| !r.2.contains("/members/")), "nothing to relay, no member lookup");
    }

    #[test]
    fn formatting_codes_need_a_permitted_role() {
        let mut t = server_with("always-receive-primary: true\n", "");
        boot(&mut t);
        let bob = join(&mut t, "Bob");
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "7", "steve", "&chello")]));
        member(&mut t, "7", &[], None);
        assert_eq!(t.said(bob), ["[#general] steve: hello"]);
        let last = t.host.spans[&Sender::Player(bob)].last().unwrap().clone();
        assert!(last.iter().all(|s| s.text != "hello" || s.style.color.as_deref() != Some("red")));
        poll(&mut t, "100", json!([msg(12, "6", "ann", "&chello")]));
        member(&mut t, "6", &["51"], None);
        assert_eq!(t.said(bob), ["[#general] ann: hello"]);
        let last = t.host.spans[&Sender::Player(bob)].last().unwrap().clone();
        assert!(last.iter().any(|s| s.text == "hello" && s.style.color.as_deref() == Some("red")), "{last:?}");
    }

    #[test]
    fn attachments_and_max_length() {
        let mut t = server_with("always-receive-primary: true\nchat:\n  discord-max-length: 20\n", "");
        boot(&mut t);
        let bob = join(&mut t, "Bob");
        prime(&mut t);
        let mut m = msg(11, "7", "steve", "look");
        m["attachments"] = json!([{"url": "https://cdn/x.png"}]);
        poll(&mut t, "100", json!([m]));
        member(&mut t, "7", &[], None);
        assert_eq!(t.said(bob), ["[#general] steve: look https://cdn/..."]);
    }

    // ----- Discord message commands -----------------------------------------------------------------

    #[test]
    fn execute_needs_an_allowed_role() {
        let mut t = server();
        boot(&mut t);
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "7", "steve", "/execute say hello")]));
        member(&mut t, "7", &["52"], None);
        let reqs = take(&mut t);
        t.tick(1);
        let reqs = [reqs, take(&mut t)].concat();
        assert_eq!(replies(&reqs), ["You do not have access to that command."]);
        assert!(t.host.console_commands.is_empty());
        poll(&mut t, "100", json!([msg(12, "6", "ann", "/execute say hello")]));
        member(&mut t, "6", &["50"], None);
        t.tick(1);
        let reqs = take(&mut t);
        assert_eq!(replies(&reqs), ["Executing command: \"/say hello\""]);
        assert_eq!(t.host.console_commands, ["say hello"]);
        // hide-command deletes the command message.
        assert!(reqs.iter().any(|r| r.1 == "DELETE" && r.2.ends_with("/channels/100/messages/12")), "{reqs:?}");
    }

    #[test]
    fn execute_runs_essentials_commands_as_the_console() {
        let mut t = server();
        boot(&mut t);
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "6", "ann", "/execute ping")]));
        member(&mut t, "6", &["50"], None);
        assert_eq!(t.host.said(Sender::Console), ["Pong!"]);
    }

    #[test]
    fn disabled_commands_say_so() {
        let mut t = server_with("commands:\n  list:\n    enabled: false\n", "");
        boot(&mut t);
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "6", "ann", "/list")]));
        member(&mut t, "6", &[], None);
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["That command is disabled!"]);
    }

    #[test]
    fn msg_and_list_commands() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "6", "ann", "/msg ali hi there")]));
        member(&mut t, "6", &[], None);
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["[me -> Alice] hi there"]);
        assert_eq!(t.said(alice), ["[ann -> me] hi there"]);
        poll(&mut t, "100", json!([msg(12, "6", "ann", "/msg nobody hi")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["Error: Player not found."]);
        t.ess.users.user(alice).set("afk", json!(true));
        poll(&mut t, "100", json!([msg(13, "6", "ann", "/msg Alice yo")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["Alice is currently AFK and may not respond.", "[me -> Alice] yo"]);
        join(&mut t, "Bob");
        poll(&mut t, "100", json!([msg(14, "6", "ann", "/list")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["There are 2 out of maximum 20 players online.\ndefault: Alice, Bob"]);
    }

    #[test]
    fn console_channel_command_relay() {
        let mut t = server_with("channels:\n  console: 300\nconsole:\n  channel: console\n  command-relay: true\n", "");
        boot(&mut t);
        poll(&mut t, "300", json!([{"id": "1", "content": "x", "author": {"id": "1", "username": "x"}}]));
        poll(&mut t, "300", json!([msg(2, "6", "ann", "say hello")]));
        assert_eq!(t.host.console_commands, ["say hello"]);
    }

    // ----- DiscordLink ------------------------------------------------------------------------------

    fn code_of(line: &str) -> String {
        line.split("/link ").nth(1).unwrap().split(' ').next().unwrap().to_string()
    }

    #[test]
    fn link_codes_and_linking_from_discord() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        let first = t.cmd(alice, "/link");
        assert_eq!(first.len(), 1);
        assert!(first[0].starts_with("To link your Minecraft account to Discord, type /link ") && first[0].ends_with(" in the Discord server."), "{first:?}");
        let code = code_of(&first[0]);
        assert_eq!(code.len(), 8);
        assert!(code.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        assert_eq!(t.cmd(alice, "/discordlink"), [format!("You already have a link code. To complete linking your Minecraft account to Discord, type /link {code} in the Discord server.")]);
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "555", "alicediscord", "/link wrongcode")]));
        t.tick(1);
        let reqs = take(&mut t);
        assert_eq!(replies(&reqs), ["Invalid linking code! Make sure you've run /link in-game and copied the code correctly."]);
        poll(&mut t, "100", json!([msg(12, "555", "alicediscord", &format!("/link {code}"))]));
        t.tick(1);
        let reqs = take(&mut t);
        assert_eq!(replies(&reqs), ["Successfully linked your account!"]);
        assert!(reqs.iter().any(|r| r.1 == "DELETE" && r.2.ends_with("/channels/100/messages/12")), "the code is hidden");
        assert_eq!(accounts(&t)[uuid_string(alice)], "555");
        assert_eq!(linked_discord_id(&mut t.ess, alice).as_deref(), Some("555"));
        assert_eq!(t.cmd(alice, "/link"), ["You have already linked your Discord account! If you wish to unlink your discord account use /unlink."]);
        poll(&mut t, "100", json!([msg(13, "555", "alicediscord", "/link abcdefgh")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["You already have an account linked! To unlink your current account, type /unlink."]);
        poll(&mut t, "100", json!([msg(14, "555", "alicediscord", "/account")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["Your account is linked to the Minecraft account: **Alice**"]);
        poll(&mut t, "100", json!([msg(15, "556", "other", "/account")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["You do not have a linked Minecraft account."]);
        poll(&mut t, "100", json!([msg(16, "556", "other", "/account <@555>")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["<@555>'s account is linked to the Minecraft account: **Alice**"]);
    }

    #[test]
    fn unlink_in_game_and_from_discord() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/unlink"), ["You do not currently have a Discord account linked to your Minecraft account."]);
        link_account(&mut t.ess, &mut t.host, alice, "555");
        assert_eq!(t.cmd(alice, "/discordunlink"), ["Unlinked your Minecraft account from all associated discord accounts."]);
        assert_eq!(accounts(&t), json!({}));
        prime(&mut t);
        poll(&mut t, "100", json!([msg(11, "555", "a", "/unlink")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["You do not currently have a Minecraft account linked to Discord!"]);
        link_account(&mut t.ess, &mut t.host, alice, "555");
        poll(&mut t, "100", json!([msg(12, "555", "a", "/unlink")]));
        t.tick(1);
        assert_eq!(replies(&take(&mut t)), ["Your Discord account has been unlinked from all associated Minecraft accounts."]);
        assert_eq!(linked_discord_id(&mut t.ess, alice), None);
    }

    #[test]
    fn accounts_survive_a_restart() {
        let mut t = server();
        let alice = join(&mut t, "Alice");
        link_account(&mut t.ess, &mut t.host, alice, "555");
        reload(&mut t.ess);
        assert_eq!(linked_discord_id(&mut t.ess, alice).as_deref(), Some("555"));
    }

    #[test]
    fn link_without_a_token() {
        let mut t = TestServer::new();
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/link"), ["Error: No token provided! Please follow the tutorial in the config in order to setup the plugin."]);
        let bob = join(&mut t, "Bob");
        assert_eq!(t.cmd(bob, "/link"), ["You do not have access to that command."]);
    }

    #[test]
    fn link_policy_kick() {
        let mut t = server_with("", "link-policy: kick\n");
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        assert_eq!(t.host.kicked.len(), 1);
        let (who, reason) = &t.host.kicked[0];
        assert_eq!(*who, alice);
        assert!(reason.starts_with("You must link your Discord account before you can join this server."), "{reason}");
        assert!(reason.contains("/link ") && reason.ends_with("https://discord.gg/abc"), "{reason}");
        let bob = join(&mut t, "Bob");
        link_account(&mut t.ess, &mut t.host, bob, "1");
        t.host.kicked.clear();
        t.quit(bob);
        join(&mut t, "Bob");
        assert!(t.host.kicked.is_empty());
    }

    #[test]
    fn link_policy_freeze() {
        let mut t = server_with("", "link-policy: freeze\n");
        boot(&mut t);
        let alice = t.join("Alice");
        t.op("Alice");
        let prompt: Vec<String> = t.said(alice).into_iter().filter(|l| l.contains("link")).collect();
        assert_eq!(prompt.len(), 1);
        assert!(prompt[0].starts_with("You must link your Discord account before you can move, chat on or interact with this server. To link your Minecraft account to Discord, type /link "), "{prompt:?}");
        assert!(prompt[0].ends_with("in this server's Discord server: https://discord.gg/abc"), "{prompt:?}");
        assert_eq!(t.chat(alice, "hi"), Chat::Cancel);
        assert_eq!(t.said(alice).len(), 1);
        let lines = t.cmd(alice, "/ping");
        assert!(lines.len() == 1 && lines[0].starts_with("You must link"), "{lines:?}");
        let start = t.loc(alice);
        let mut moved = start.clone();
        moved.x += 3.0;
        t.walk(alice, moved);
        t.tick(1);
        assert_eq!(t.loc(alice).x, start.x);
        assert_eq!(t.ess.on_interact(&mut t.host, alice, None, None), Allow::No);
        link_account(&mut t.ess, &mut t.host, alice, "555");
        assert_ne!(t.chat(alice, "hi"), Chat::Cancel);
        assert_eq!(t.cmd(alice, "/ping"), ["Pong!"]);
    }

    #[test]
    fn block_unlinked_chat() {
        let mut t = server_with("", "block-unlinked-chat: true\n");
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.tick(1);
        take(&mut t);
        t.chat(alice, "hi");
        t.tick(1);
        assert!(posts_to(&take(&mut t), "/messages").is_empty());
        link_account(&mut t.ess, &mut t.host, alice, "555");
        take(&mut t);
        t.chat(alice, "hi");
        t.tick(1);
        assert_eq!(posts_to(&take(&mut t), "/channels/100/messages").len(), 1);
    }

    #[test]
    fn role_sync_both_ways() {
        let mut t = server_with("", "role-sync:\n  groups:\n    admin: 777\n    default: 0\n  roles:\n    888: vip\n");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: []\n  admin:\n    permissions: []\n  vip:\n    permissions: [essentials.fly]\nusers:\n  alice:\n    groups: [admin]\n");
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        t.op("Alice");
        t.tick(1);
        take(&mut t);
        link_account(&mut t.ess, &mut t.host, alice, "555");
        member(&mut t, "555", &["888"], None);
        let reqs = take(&mut t);
        assert!(reqs.iter().any(|r| r.1 == "PUT" && r.2.ends_with("/guilds/1/members/555/roles/777")), "{reqs:?}");
        assert_eq!(t.ess.perms.groups_of(alice, "Alice"), ["admin", "vip"], "vip group from the Discord role");
        // Unlinking removes the synced roles and groups.
        t.cmd(alice, "/unlink");
        member(&mut t, "555", &["888", "777"], None);
        let reqs = take(&mut t);
        assert!(reqs.iter().any(|r| r.1 == "DELETE" && r.2.ends_with("/guilds/1/members/555/roles/777")), "{reqs:?}");
        assert_eq!(t.ess.perms.groups_of(alice, "Alice"), ["admin"]);
    }

    #[test]
    fn mail_is_relayed_to_linked_accounts() {
        let mut t = server();
        boot(&mut t);
        let alice = join(&mut t, "Alice");
        link_account(&mut t.ess, &mut t.host, alice, "555");
        t.tick(1);
        take(&mut t);
        relay_mail(&mut t.ess, &mut t.host, alice, "Bob", "see *you*");
        t.tick(1);
        let reqs = take(&mut t);
        let dm = reqs.iter().find(|r| r.1 == "POST" && r.2.ends_with("/users/@me/channels")).expect("DM channel");
        assert_eq!(dm.3["recipient_id"], "555");
        answer(&mut t, dm.0, 200, json!({"id": "4242"}));
        t.tick(2);
        let sent = posts_to(&take(&mut t), "/channels/4242/messages");
        assert_eq!(sent[0]["content"], "**New mail from Bob:** see \\*you\\*");
    }

    #[test]
    fn invalid_token_disables_the_module() {
        let mut t = server();
        t.tick(1);
        let reqs = take(&mut t);
        let me = reqs.iter().find(|r| r.2.ends_with("/users/@me")).unwrap();
        answer(&mut t, me.0, 401, json!({"message": "401: Unauthorized"}));
        t.tick(1);
        let console = t.host.said(Sender::Console);
        assert!(console.iter().any(|l| l.starts_with("An error occurred while logging into Discord")), "{console:?}");
        let alice = join(&mut t, "Alice");
        take(&mut t);
        t.chat(alice, "hi");
        t.tick(200);
        assert!(take(&mut t).is_empty());
    }

    #[test]
    fn foreign_http_ids_are_not_claimed() {
        let mut t = server();
        assert!(!on_http(&mut t.ess, &mut t.host, 7, 200, "{}"));
        assert!(on_http(&mut t.ess, &mut t.host, HTTP_ID | 12345, 200, "{}"));
    }

    #[test]
    fn mini_regex_filters() {
        assert!(Regex::new("^[!?]").find("!x"));
        assert!(!Regex::new("^[!?]").find("x!"));
        assert!(Regex::new("a.c").find("zzabcz"));
        assert!(Regex::new("^ab*c$").find("abbbc"));
        assert!(!Regex::new("^ab+c$").find("ac"));
        assert!(Regex::new("colou?r").find("color"));
        assert!(Regex::new("\\d+").find("abc123"));
        assert!(Regex::new("cat|dog").find("hotdog"));
        assert!(Regex::new("[^a-z]").find("abc1"));
        assert!(!Regex::new("[^a-z]").find("abc"));
    }
}
