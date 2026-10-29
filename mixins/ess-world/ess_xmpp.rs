// horizon-essentials area `world`, module `ess_xmpp` (pure logic, standalone-tested): EssentialsX XMPP: /setxmpp, /xmpp, /xmppspy and a minimal XMPP client.
//
// EssentialsXMPP links players to XMPP (Jabber) addresses (xmpp/users.yml: `<name>: {address, spy}`), relays
// joins, quits and chat to "spy" addresses, lets XMPP users chat into the game (`@player msg` whispers,
// `/command` for op-users) and /xmpp messages an address. The client is ours (EssentialsX uses Smack): RFC 6120
// stream, SASL PLAIN (or legacy iq:auth with sasl-enabled false), resource binding, session, presence and
// chat messages over the Host's TCP methods, driven by a tick listener. The Host's TCP has no TLS, so servers
// that require STARTTLS refuse us: the reason is logged and XMPP stays off.

use crate::game::ess_core::*;
use crate::game::ess_text::{escape_tags, legacy_to_mini, strip_legacy};
use serde_json::{Map, Value, json};

/// TCP connection ids this module opens carry this tag in their high bits.
pub const TCP_ID: u64 = 0x584D_0000_0000_0000;
const RESOURCE: &str = "Essentials-XMPP";
/// Ticks before reconnecting after the server dropped an established connection.
const RECONNECT_TICKS: u64 = 600;

pub fn register(ess: &mut Ess) {
    ess.register(Command { name: "setxmpp", run: setxmpp, tab: None, usages: &[] });
    ess.register(Command { name: "xmpp", run: xmpp, tab: None, usages: &[] });
    ess.register(Command { name: "xmppspy", run: xmppspy, tab: None, usages: &[] });
    ess.listeners.tick.push(on_tick);
    ess.listeners.join.push(on_join);
    ess.listeners.quit.push(on_quit);
    ess.listeners.chat.push(on_chat);
}

// ===== State =======================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for the first stream features (login).
    Features,
    SaslSent,
    IqAuthSent,
    /// After SASL success: waiting for the restarted stream's features (bind).
    BindFeatures,
    BindSent,
    SessionSent,
    Online,
}

pub struct Conn {
    pub id: u64,
    pub phase: Phase,
    buf: Vec<u8>,
    session: bool,
    /// Our full JID once bound.
    pub jid: String,
}

#[derive(Default)]
pub struct XmppState {
    started: bool,
    pub config: Value,
    pub conn: Option<Conn>,
    next_id: u64,
    /// Reconnect at this scheduler tick (after the server dropped us).
    reconnect_at: Option<u64>,
}

fn cfg_str(cfg: &Value, path: &str) -> Option<String> {
    crate::game::ess_yaml::get(cfg, path).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}
fn cfg_list(cfg: &Value, path: &str) -> Vec<String> {
    match crate::game::ess_yaml::get(cfg, path) {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn log(ess: &Ess, host: &mut dyn Host, line: &str) {
    host.send(Sender::Console, &ess.render(&escape_tags(line)));
}

/// sender.sendMessage(legacy text).
fn send_plain(ess: &Ess, host: &mut dyn Host, to: Sender, text: &str) {
    ess.send_mini(host, to, &legacy_to_mini(&escape_tags(text), false));
}

fn load_config(ess: &mut Ess) -> Value {
    let mut config = crate::game::ess_defaults::file("xmpp/config.yml")
        .and_then(|t| crate::game::ess_yaml::parse(t).ok())
        .unwrap_or(json!({}));
    let file = ess.read_yaml("xmpp/config.yml");
    if file.is_object() {
        merge(&mut config, file);
    }
    config
}

/// XMPPManager.isConfigValid: a server other than the sample's.
pub fn config_valid(config: &Value) -> bool {
    cfg_str(config, "xmpp.server").is_some_and(|s| !s.is_empty() && s != "example.com")
}

// ===== Users (UserManager: xmpp/users.yml) ========================================================

fn users(ess: &Ess) -> Map<String, Value> {
    match ess.read_yaml("xmpp/users.yml") {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

pub fn address_of(ess: &Ess, username: &str) -> Option<String> {
    users(ess).get(&username.to_lowercase())?.get("address")?.as_str().map(str::to_string)
}

pub fn is_spy(ess: &Ess, username: &str) -> bool {
    users(ess).get(&username.to_lowercase()).and_then(|u| u.get("spy")).and_then(Value::as_bool).unwrap_or(false)
}

fn set_user(ess: &Ess, username: &str, address: Option<String>, spy: bool) {
    let mut all = users(ess);
    all.insert(username.to_lowercase(), json!({"address": address, "spy": spy}));
    ess.write_yaml("xmpp/users.yml", &Value::Object(all));
}

/// UserManager.getUserByAddress: the (lower-case) user name linked to an address.
pub fn user_by_address(ess: &Ess, address: &str) -> Option<String> {
    users(ess).into_iter().find(|(_, u)| u.get("address").and_then(Value::as_str).is_some_and(|a| a.eq_ignore_ascii_case(address))).map(|(k, _)| k)
}

/// The addresses of users with spy on.
pub fn spy_users(ess: &Ess) -> Vec<String> {
    users(ess)
        .values()
        .filter(|u| u.get("spy").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|u| u.get("address").and_then(Value::as_str).map(str::to_string))
        .collect()
}

// ===== Commands ====================================================================================

fn setxmpp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let player = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let name = ctx.name_of(player);
    let spy = is_spy(ctx.ess, &name);
    set_user(ctx.ess, &name, Some(args[0].clone()), spy);
    let to = ctx.sender;
    send_plain(ctx.ess, ctx.host, to, &format!("XMPP address set to {}", args[0]));
    Ok(())
}

fn xmpp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let to = ctx.sender;
    let Some(address) = address_of(ctx.ess, &args[0]) else {
        ctx.send_mini("<red>There are no players matching that name.");
        return Ok(());
    };
    let message = final_arg(args, 1);
    let sender_name = match ctx.sender {
        Sender::Player(p) => ctx.name_of(p),
        Sender::Console => crate::game::ess_text::plain(&ctx.ess.render(&ctx.ess.tl("consoleName", &[]))),
    };
    send_plain(ctx.ess, ctx.host, to, &format!("[{sender_name}>{address}] {message}"));
    if !send_message(ctx.ess, ctx.host, &address, &format!("[{sender_name}] {message}")) {
        ctx.send_mini("<red>Error sending message.");
    }
    Ok(())
}

fn xmppspy(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let targets: Vec<Uuid> = match args[0].as_str() {
        "" => return Err(CmdError::PlayerNotFound),
        "@s" | "@p" if ctx.is_player() => vec![ctx.player()?],
        "*" | "**" => {
            let mut out = Vec::new();
            for p in ctx.host.online() {
                if ctx.ess.can_see(ctx.host, ctx.sender, p) {
                    out.push(p);
                }
            }
            out
        }
        term => vec![ctx.online_player(term)?],
    };
    for p in targets {
        let name = ctx.name_of(p);
        let spy = !is_spy(ctx.ess, &name);
        let address = address_of(ctx.ess, &name);
        set_user(ctx.ess, &name, address, spy);
        let to = ctx.sender;
        send_plain(ctx.ess, ctx.host, to, &format!("XMPP Spy {} for {name}", if spy { "enabled" } else { "disabled" }));
    }
    Ok(())
}

// ===== Events (EssentialsXMPPPlayerListener) =========================================================

fn on_join(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _first: bool) {
    let name = host.name(player).unwrap_or_default();
    ess.scheduler.later(0, Box::new(update_presence));
    spy_broadcast(ess, &format!("Player {name} joined the game"));
}

fn on_quit(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    let name = host.name(player).unwrap_or_default();
    ess.scheduler.later(0, Box::new(update_presence));
    spy_broadcast(ess, &format!("Player {name} left the game"));
}

/// Chat goes to the spies in Bukkit's default chat format (`<%1$s> %2$s`); the chat itself is left alone.
fn on_chat(ess: &mut Ess, host: &mut dyn Host, player: Uuid, message: &str) -> Chat {
    let name = host.name(player).unwrap_or_default();
    spy_broadcast(ess, &format!("<{name}> {message}"));
    Chat::Pass
}

/// sendMessageToSpyUsers: one message per spy address, on the next tick.
fn spy_broadcast(ess: &mut Ess, message: &str) {
    for address in spy_users(ess) {
        let message = message.to_string();
        ess.scheduler.later(0, Box::new(move |ess, host| {
            send_message(ess, host, &address, &message);
        }));
    }
}

/// XMPPManager.updatePresence: the online count as our status.
fn update_presence(ess: &mut Ess, host: &mut dyn Host) {
    let count = host.online().len();
    let (status, show) = match count {
        0 => ("No one online.".to_string(), Some("dnd")),
        1 => ("1 player online.".to_string(), None),
        n => (format!("{n} players online."), None),
    };
    let Some(id) = online_id(ess) else { return };
    host.tcp_send(id, presence(&status, show).as_bytes());
}

fn online_id(ess: &mut Ess) -> Option<u64> {
    ess.state::<XmppState>().conn.as_ref().filter(|c| c.phase == Phase::Online).map(|c| c.id)
}

/// XMPPManager.sendMessage: false when we aren't connected (or there is no address).
pub fn send_message(ess: &mut Ess, host: &mut dyn Host, address: &str, message: &str) -> bool {
    if address.is_empty() {
        return false;
    }
    let Some(id) = online_id(ess) else { return false };
    let body = strip_legacy(message, false);
    host.tcp_send(id, format!("<message to='{}' type='chat'><body>{}</body></message>", escape(address), escape(&body)).as_bytes());
    true
}

// ===== Connection ==================================================================================

fn on_tick(ess: &mut Ess, host: &mut dyn Host) {
    if !ess.state::<XmppState>().started {
        ess.state::<XmppState>().started = true;
        let config = load_config(ess);
        ess.state::<XmppState>().config = config.clone();
        // Off (and quiet) until configured: EssentialsX warns `xmppNotConfigured` here, which every server
        // without XMPP would print.
        if config_valid(&config) {
            connect(ess, host);
        }
    }
    let now = ess.scheduler.tick;
    if ess.state::<XmppState>().reconnect_at.is_some_and(|at| now >= at) {
        ess.state::<XmppState>().reconnect_at = None;
        connect(ess, host);
    }
    if ess.state::<XmppState>().conn.is_none() {
        return;
    }
    for (id, data) in host.tcp_poll() {
        let ours = ess.state::<XmppState>().conn.as_ref().is_some_and(|c| c.id == id);
        if !ours {
            continue;
        }
        if data.is_empty() {
            let was_online = ess.state::<XmppState>().conn.as_ref().is_some_and(|c| c.phase == Phase::Online);
            ess.state::<XmppState>().conn = None;
            log(ess, host, "Disconnected from the XMPP server.");
            if was_online {
                ess.state::<XmppState>().reconnect_at = Some(now + RECONNECT_TICKS);
            }
            continue;
        }
        if let Some(c) = ess.state::<XmppState>().conn.as_mut() {
            c.buf.extend_from_slice(&data);
        }
        loop {
            let event = {
                let Some(c) = ess.state::<XmppState>().conn.as_mut() else { break };
                let Some((event, used)) = next_event(&c.buf) else { break };
                c.buf.drain(..used);
                event
            };
            handle(ess, host, event);
        }
    }
}

fn connect(ess: &mut Ess, host: &mut dyn Host) {
    let config = ess.state::<XmppState>().config.clone();
    let server = cfg_str(&config, "xmpp.server").unwrap_or_default();
    let port: u16 = cfg_str(&config, "xmpp.port").and_then(|p| p.parse().ok()).unwrap_or(5222);
    let user = cfg_str(&config, "xmpp.user").unwrap_or_default();
    log(ess, host, &format!("Connecting to xmpp server {server}:{port} as user {user}."));
    let id = {
        let st = ess.state::<XmppState>();
        st.next_id += 1;
        TCP_ID | st.next_id
    };
    host.tcp_open(id, &server, port, false);
    ess.state::<XmppState>().conn = Some(Conn { id, phase: Phase::Features, buf: Vec::new(), session: false, jid: String::new() });
    host.tcp_send(id, stream_header(&service(&config)).as_bytes());
}

fn service(config: &Value) -> String {
    cfg_str(config, "xmpp.servicename").or_else(|| cfg_str(config, "xmpp.server")).unwrap_or_default()
}

fn stream_header(to: &str) -> String {
    format!("<?xml version='1.0'?><stream:stream to='{}' xmlns='jabber:client' xmlns:stream='http://etherx.jabber.org/streams' version='1.0'>", escape(to))
}

fn presence(status: &str, show: Option<&str>) -> String {
    let show = show.map(|s| format!("<show>{s}</show>")).unwrap_or_default();
    format!("<presence>{show}<status>{}</status><priority>2</priority></presence>", escape(status))
}

/// Gives up on the server (login or TLS failure): logged as Smack's connect() failure.
fn fail(ess: &mut Ess, host: &mut dyn Host, reason: &str) {
    let server = cfg_str(&ess.state::<XmppState>().config, "xmpp.server").unwrap_or_default();
    log(ess, host, &format!("Failed to connect to server: {server}"));
    log(ess, host, reason);
    if let Some(c) = ess.state::<XmppState>().conn.take() {
        host.tcp_close(c.id);
    }
}

fn send(ess: &mut Ess, host: &mut dyn Host, xml: &str) {
    if let Some(id) = ess.state::<XmppState>().conn.as_ref().map(|c| c.id) {
        host.tcp_send(id, xml.as_bytes());
    }
}

fn set_phase(ess: &mut Ess, phase: Phase) {
    if let Some(c) = ess.state::<XmppState>().conn.as_mut() {
        c.phase = phase;
    }
}

fn handle(ess: &mut Ess, host: &mut dyn Host, event: Event) {
    let Some(phase) = ess.state::<XmppState>().conn.as_ref().map(|c| c.phase) else { return };
    let config = ess.state::<XmppState>().config.clone();
    let el = match event {
        Event::StreamOpen => return,
        Event::StreamClose => {
            if let Some(c) = ess.state::<XmppState>().conn.take() {
                host.tcp_close(c.id);
            }
            log(ess, host, "Disconnected from the XMPP server.");
            return;
        }
        Event::Stanza(el) => el,
    };
    let user = cfg_str(&config, "xmpp.user").unwrap_or_default();
    let password = cfg_str(&config, "xmpp.password").unwrap_or_default();
    match (el.name.as_str(), phase) {
        ("features", Phase::Features) => {
            let tls_required = el.child("starttls").is_some_and(|s| s.child("required").is_some());
            if tls_required || crate::game::ess_yaml::get(&config, "xmpp.require-server-tls").and_then(Value::as_bool).unwrap_or(false) {
                fail(ess, host, "The server requires TLS (STARTTLS), which Horizon's network layer cannot provide: XMPP stays off.");
                return;
            }
            let plain = el.child("mechanisms").is_some_and(|m| m.children.iter().any(|c| c.name == "mechanism" && c.text.trim() == "PLAIN"));
            let iq_auth = el.child("auth").is_some();
            let sasl = crate::game::ess_yaml::get(&config, "xmpp.sasl-enabled").and_then(Value::as_bool).unwrap_or(false);
            if plain && (sasl || !iq_auth) {
                let token = base64(format!("\0{user}\0{password}").as_bytes());
                send(ess, host, &format!("<auth xmlns='urn:ietf:params:xml:ns:xmpp-sasl' mechanism='PLAIN'>{token}</auth>"));
                set_phase(ess, Phase::SaslSent);
            } else if iq_auth || !plain && el.child("mechanisms").is_none() {
                send(
                    ess,
                    host,
                    &format!(
                        "<iq type='set' id='auth_1'><query xmlns='jabber:iq:auth'><username>{}</username><password>{}</password><resource>{RESOURCE}</resource></query></iq>",
                        escape(&user),
                        escape(&password)
                    ),
                );
                set_phase(ess, Phase::IqAuthSent);
            } else {
                fail(ess, host, "The server offers no login mechanism we support (SASL PLAIN or iq:auth).");
            }
        }
        ("success", Phase::SaslSent) => {
            let header = stream_header(&service(&config));
            send(ess, host, &header);
            set_phase(ess, Phase::BindFeatures);
        }
        ("failure", _) => fail(ess, host, "Authentication failed: check xmpp.user and xmpp.password."),
        ("features", Phase::BindFeatures) => {
            let session = el.child("session").is_some_and(|s| s.child("optional").is_none());
            if let Some(c) = ess.state::<XmppState>().conn.as_mut() {
                c.session = session;
            }
            send(ess, host, &format!("<iq type='set' id='bind_1'><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'><resource>{RESOURCE}</resource></bind></iq>"));
            set_phase(ess, Phase::BindSent);
        }
        ("iq", _) => {
            let kind = el.attr("type").unwrap_or("");
            let id = el.attr("id").unwrap_or("").to_string();
            match (kind, phase, id.as_str()) {
                ("result", Phase::BindSent, "bind_1") => {
                    let jid = el.child("bind").and_then(|b| b.child("jid")).map(|j| j.text.clone()).unwrap_or_default();
                    let session = ess.state::<XmppState>().conn.as_mut().map(|c| {
                        c.jid = jid;
                        c.session
                    });
                    if session == Some(true) {
                        send(ess, host, "<iq type='set' id='sess_1'><session xmlns='urn:ietf:params:xml:ns:xmpp-session'/></iq>");
                        set_phase(ess, Phase::SessionSent);
                    } else {
                        go_online(ess, host);
                    }
                }
                ("result", Phase::SessionSent, "sess_1") | ("result", Phase::IqAuthSent, "auth_1") => go_online(ess, host),
                ("error", Phase::IqAuthSent | Phase::BindSent | Phase::SessionSent, _) => fail(ess, host, "Authentication failed: check xmpp.user and xmpp.password."),
                ("get" | "set", _, _) => {
                    let to = el.attr("from").map(|f| format!(" to='{}'", escape(f))).unwrap_or_default();
                    let reply = if el.child("ping").is_some() {
                        format!("<iq type='result' id='{}'{to}/>", escape(&id))
                    } else {
                        format!(
                            "<iq type='error' id='{}'{to}><error type='cancel'><service-unavailable xmlns='urn:ietf:params:xml:ns:xmpp-stanzas'/></error></iq>",
                            escape(&id)
                        )
                    };
                    send(ess, host, &reply);
                }
                _ => {}
            }
        }
        ("presence", Phase::Online) => {
            // Roster subscription mode accept_all.
            if el.attr("type") == Some("subscribe") {
                if let Some(from) = el.attr("from") {
                    let reply = format!("<presence to='{}' type='subscribed'/>", escape(from));
                    send(ess, host, &reply);
                }
            }
        }
        ("message", Phase::Online) => process_message(ess, host, &el),
        _ => {}
    }
}

fn go_online(ess: &mut Ess, host: &mut dyn Host) {
    set_phase(ess, Phase::Online);
    send(ess, host, &presence("No one online.", None));
}

/// The address without its resource (StringUtils.parseBareAddress).
fn bare(jid: &str) -> &str {
    jid.split('/').next().unwrap_or(jid)
}

/// XMPPManager.processMessage: `@player text` whispers, `/command` from op-users runs as the console, anything
/// else is chat from the linked player.
fn process_message(ess: &mut Ess, host: &mut dyn Host, el: &Element) {
    if el.attr("type") == Some("error") {
        return;
    }
    let message = el.child("body").map(|b| b.text.clone()).unwrap_or_default();
    if message.is_empty() {
        return;
    }
    let from = bare(el.attr("from").unwrap_or("")).to_string();
    match message.chars().next() {
        Some('@') => {
            let mut parts = message.splitn(2, ' ');
            let (target, rest) = (parts.next().unwrap_or(""), parts.next());
            if rest.is_none() {
                return;
            }
            let matches = match_players(host, &target[1..]);
            if matches.is_empty() {
                send_message(ess, host, &from, &format!("User {target} not found"));
            } else {
                let sender = user_by_address(ess, &from).unwrap_or_else(|| "null".into());
                for p in matches {
                    let name = host.name(p).unwrap_or_default();
                    send_plain(ess, host, Sender::Player(p), &format!("[{sender}>{name}]  {message}"));
                }
            }
        }
        Some('/') => {
            let ops = cfg_list(&ess.state::<XmppState>().config, "op-users");
            if ops.iter().any(|o| o == &from) {
                let line = message[1..].to_string();
                ess.scheduler.later(0, Box::new(move |ess, host| {
                    if !ess.dispatch(host, Sender::Console, &line) {
                        host.console_command(&line);
                    }
                }));
            }
        }
        _ => {
            // EssentialsX fails on messages from unlinked addresses; they are dropped here.
            let Some(username) = user_by_address(ess, &from) else { return };
            let display = ess.users.names.get(&username).copied().and_then(|u| host.name(u)).unwrap_or(username);
            let line = format!("={display}: {message}");
            for p in host.online() {
                send_plain(ess, host, Sender::Player(p), &line);
            }
            for address in spy_users(ess) {
                if !address.eq_ignore_ascii_case(&from) {
                    send_message(ess, host, &address, &line);
                }
            }
        }
    }
}

/// Bukkit's Server.matchPlayer: the exact name, else every name containing the text.
fn match_players(host: &mut dyn Host, term: &str) -> Vec<Uuid> {
    let term = term.to_lowercase();
    let online: Vec<(Uuid, String)> = host.online().into_iter().map(|p| (p, host.name(p).unwrap_or_default())).collect();
    if let Some((p, _)) = online.iter().find(|(_, n)| n.to_lowercase() == term) {
        return vec![*p];
    }
    online.into_iter().filter(|(_, n)| n.to_lowercase().contains(&term)).map(|(p, _)| p).collect()
}

// ===== XML ===========================================================================================

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Element {
    /// Local name (prefix dropped: `stream:features` is `features`).
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }
}

#[derive(Debug, PartialEq)]
pub enum Event {
    StreamOpen,
    StreamClose,
    Stanza(Element),
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "&apos;").replace('"', "&quot;")
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let ch = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "apos" => Some('\''),
            "quot" => Some('"'),
            e if e.starts_with("#x") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_string()
}

enum Parse {
    Incomplete,
    Invalid,
}

/// The next complete top-level event in the buffer and the bytes it used; None until more bytes arrive.
pub fn next_event(buf: &[u8]) -> Option<(Event, usize)> {
    let valid = match std::str::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => std::str::from_utf8(&buf[..e.valid_up_to()]).ok()?,
    };
    let s = valid;
    let mut i = 0;
    loop {
        // Whitespace keepalives and prolog.
        let trimmed = s[i..].trim_start();
        i = s.len() - trimmed.len();
        let rest = &s[i..];
        if rest.is_empty() {
            return None;
        }
        if rest.starts_with("<?") {
            let end = rest.find("?>")?;
            i += end + 2;
            continue;
        }
        if rest.starts_with("<!--") {
            let end = rest.find("-->")?;
            i += end + 3;
            continue;
        }
        if !rest.starts_with('<') {
            // Stray text at the top level: skip to the next tag.
            match rest.find('<') {
                Some(n) => {
                    i += n;
                    continue;
                }
                None => return None,
            }
        }
        if rest.starts_with("</") {
            let end = rest.find('>')?;
            return Some((Event::StreamClose, i + end + 1));
        }
        let name_end = rest[1..].find(|c: char| c.is_whitespace() || c == '>' || c == '/').map(|n| n + 1)?;
        if local(&rest[1..name_end]) == "stream" {
            let end = tag_end(rest)?;
            return Some((Event::StreamOpen, i + end + 1));
        }
        return match parse_element(rest, 0) {
            Ok((el, used)) => Some((Event::Stanza(el), i + used)),
            Err(Parse::Incomplete) => None,
            Err(Parse::Invalid) => {
                // Skip the broken tag rather than stall.
                let end = tag_end(rest).unwrap_or(rest.len() - 1);
                Some((Event::StreamOpen, i + end + 1))
            }
        };
    }
}

/// The index of the '>' closing the tag that starts at 0 (quotes respected).
fn tag_end(s: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

fn parse_element(s: &str, start: usize) -> Result<(Element, usize), Parse> {
    let b = s.as_bytes();
    let mut i = start + 1;
    let name_start = i;
    while i < b.len() && !(b[i] as char).is_whitespace() && b[i] != b'>' && b[i] != b'/' {
        i += 1;
    }
    if i >= b.len() {
        return Err(Parse::Incomplete);
    }
    let mut el = Element { name: local(&s[name_start..i]), ..Element::default() };
    // Attributes.
    loop {
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return Err(Parse::Incomplete);
        }
        if b[i] == b'/' {
            if i + 1 >= b.len() {
                return Err(Parse::Incomplete);
            }
            return if b[i + 1] == b'>' { Ok((el, i + 2)) } else { Err(Parse::Invalid) };
        }
        if b[i] == b'>' {
            i += 1;
            break;
        }
        let key_start = i;
        while i < b.len() && b[i] != b'=' && !(b[i] as char).is_whitespace() {
            i += 1;
        }
        let key = s[key_start..i].to_string();
        while i < b.len() && (b[i] == b'=' || (b[i] as char).is_whitespace()) {
            i += 1;
        }
        if i >= b.len() {
            return Err(Parse::Incomplete);
        }
        let q = b[i];
        if q != b'"' && q != b'\'' {
            return Err(Parse::Invalid);
        }
        i += 1;
        let v_start = i;
        while i < b.len() && b[i] != q {
            i += 1;
        }
        if i >= b.len() {
            return Err(Parse::Incomplete);
        }
        el.attrs.push((local_attr(&key), unescape(&s[v_start..i])));
        i += 1;
    }
    // Content.
    loop {
        if i >= b.len() {
            return Err(Parse::Incomplete);
        }
        let rest = &s[i..];
        if rest.starts_with("</") {
            let end = rest.find('>').ok_or(Parse::Incomplete)?;
            return Ok((el, i + end + 1));
        }
        if rest.starts_with("<!--") {
            let end = rest.find("-->").ok_or(Parse::Incomplete)?;
            i += end + 3;
        } else if rest.starts_with("<![CDATA[") {
            let end = rest.find("]]>").ok_or(Parse::Incomplete)?;
            el.text.push_str(&rest[9..end]);
            i += end + 3;
        } else if rest.starts_with('<') {
            let (child, next) = parse_element(s, i)?;
            el.children.push(child);
            i = next;
        } else {
            let end = rest.find('<').ok_or(Parse::Incomplete)?;
            el.text.push_str(&unescape(&rest[..end]));
            i += end;
        }
    }
}

/// Attribute names keep `xml:lang` style prefixes except namespace declarations' (kept as is).
fn local_attr(name: &str) -> String {
    name.to_string()
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ess_testkit::*;

    const CONFIG: &str = "xmpp:\n  server: 'chat.example.org'\n  user: 'bot'\n  password: 'secret'\n  sasl-enabled: true\nop-users:\n  - 'admin@example.org'\n";

    fn server(config: &str) -> TestServer {
        let t = TestServer::new();
        std::fs::create_dir_all(t.ess.dir.join("xmpp")).unwrap();
        if !config.is_empty() {
            std::fs::write(t.ess.dir.join("xmpp/config.yml"), config).unwrap();
        }
        t
    }

    fn sent(t: &mut TestServer, id: u64) -> String {
        String::from_utf8(t.host.tcp_sent.remove(&id).unwrap_or_default()).unwrap()
    }

    fn feed(t: &mut TestServer, id: u64, xml: &str) {
        t.host.tcp_incoming.push((id, xml.as_bytes().to_vec()));
        t.tick(1);
    }

    const OPEN: &str = "<?xml version='1.0'?><stream:stream xmlns='jabber:client' xmlns:stream='http://etherx.jabber.org/streams' id='s1' from='chat.example.org' version='1.0'>";

    /// Connects and logs in; returns the connection id with the sent buffer drained.
    fn online(t: &mut TestServer) -> u64 {
        t.tick(1);
        let (id, host, port, tls) = t.host.tcp_opened[0].clone();
        assert_eq!((host.as_str(), port, tls), ("chat.example.org", 5222, false));
        assert!(sent(t, id).contains("<stream:stream to='chat.example.org'"));
        feed(t, id, &format!("{OPEN}<stream:features><mechanisms xmlns='urn:ietf:params:xml:ns:xmpp-sasl'><mechanism>SCRAM-SHA-1</mechanism><mechanism>PLAIN</mechanism></mechanisms></stream:features>"));
        assert_eq!(sent(t, id), "<auth xmlns='urn:ietf:params:xml:ns:xmpp-sasl' mechanism='PLAIN'>AGJvdABzZWNyZXQ=</auth>");
        feed(t, id, "<success xmlns='urn:ietf:params:xml:ns:xmpp-sasl'/>");
        assert!(sent(t, id).contains("<stream:stream to='chat.example.org'"));
        feed(t, id, &format!("{OPEN}<stream:features><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'/><session xmlns='urn:ietf:params:xml:ns:xmpp-session'/></stream:features>"));
        assert_eq!(sent(t, id), "<iq type='set' id='bind_1'><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'><resource>Essentials-XMPP</resource></bind></iq>");
        feed(t, id, "<iq type='result' id='bind_1'><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'><jid>bot@chat.example.org/Essentials-XMPP</jid></bind></iq>");
        assert_eq!(sent(t, id), "<iq type='set' id='sess_1'><session xmlns='urn:ietf:params:xml:ns:xmpp-session'/></iq>");
        feed(t, id, "<iq type='result' id='sess_1'/>");
        assert_eq!(sent(t, id), "<presence><status>No one online.</status><priority>2</priority></presence>");
        assert_eq!(t.ess.state::<XmppState>().conn.as_ref().unwrap().jid, "bot@chat.example.org/Essentials-XMPP");
        id
    }

    #[test]
    fn unconfigured_xmpp_stays_off_and_quiet() {
        let mut t = server("");
        let alice = t.join("Alice");
        t.tick(5);
        assert!(t.host.tcp_opened.is_empty());
        assert!(t.host.said(Sender::Console).is_empty());
        t.cmd(alice, "/setxmpp alice@example.org");
        assert_eq!(t.cmd(alice, "/xmpp alice hi"), ["[Alice>alice@example.org] hi", "Error sending message."]);
    }

    #[test]
    fn logs_in_with_sasl_plain_and_binds() {
        let mut t = server(CONFIG);
        online(&mut t);
        let console = t.host.said(Sender::Console);
        assert_eq!(console[0], "Connecting to xmpp server chat.example.org:5222 as user bot.");
    }

    #[test]
    fn legacy_iq_auth_without_sasl_and_split_packets() {
        let mut t = server("xmpp:\n  server: 'old.example.org'\n  port: 5223\n  servicename: 'example.org'\n  user: 'bot'\n  password: 'p&w'\n");
        t.tick(1);
        let (id, host, port, _) = t.host.tcp_opened[0].clone();
        assert_eq!((host.as_str(), port), ("old.example.org", 5223));
        assert!(sent(&mut t, id).contains("to='example.org'"));
        // The features arrive in two pieces.
        let xml = format!("{OPEN}<stream:features><auth xmlns='http://jabber.org/features/iq-auth'/><mechanisms xmlns='urn:ietf:params:xml:ns:xmpp-sasl'><mechanism>PLAIN</mechanism></mechanisms></stream:features>");
        let (a, b) = xml.split_at(xml.len() - 30);
        feed(&mut t, id, a);
        assert_eq!(sent(&mut t, id), "");
        feed(&mut t, id, b);
        assert_eq!(
            sent(&mut t, id),
            "<iq type='set' id='auth_1'><query xmlns='jabber:iq:auth'><username>bot</username><password>p&amp;w</password><resource>Essentials-XMPP</resource></query></iq>"
        );
        feed(&mut t, id, "<iq type='result' id='auth_1'/>");
        assert_eq!(sent(&mut t, id), "<presence><status>No one online.</status><priority>2</priority></presence>");
    }

    #[test]
    fn starttls_required_is_refused_with_a_reason() {
        let mut t = server(CONFIG);
        t.tick(1);
        let id = t.host.tcp_opened[0].0;
        t.host.said(Sender::Console);
        feed(&mut t, id, &format!("{OPEN}<stream:features><starttls xmlns='urn:ietf:params:xml:ns:xmpp-tls'><required/></starttls><mechanisms><mechanism>PLAIN</mechanism></mechanisms></stream:features>"));
        assert_eq!(
            t.host.said(Sender::Console),
            [
                "Failed to connect to server: chat.example.org",
                "The server requires TLS (STARTTLS), which Horizon's network layer cannot provide: XMPP stays off."
            ]
        );
        assert_eq!(t.host.tcp_closed, [id]);
        assert!(t.ess.state::<XmppState>().conn.is_none());
        // require-server-tls in our config refuses even an optional STARTTLS.
        let mut t = server(&format!("{CONFIG}").replace("sasl-enabled: true", "require-server-tls: true"));
        t.tick(1);
        let id = t.host.tcp_opened[0].0;
        feed(&mut t, id, &format!("{OPEN}<stream:features><mechanisms><mechanism>PLAIN</mechanism></mechanisms></stream:features>"));
        assert_eq!(t.host.tcp_closed, [id]);
    }

    #[test]
    fn auth_failure_is_logged() {
        let mut t = server(CONFIG);
        t.tick(1);
        let id = t.host.tcp_opened[0].0;
        feed(&mut t, id, &format!("{OPEN}<stream:features><mechanisms><mechanism>PLAIN</mechanism></mechanisms></stream:features>"));
        t.host.said(Sender::Console);
        feed(&mut t, id, "<failure xmlns='urn:ietf:params:xml:ns:xmpp-sasl'><not-authorized/></failure>");
        assert_eq!(
            t.host.said(Sender::Console),
            ["Failed to connect to server: chat.example.org", "Authentication failed: check xmpp.user and xmpp.password."]
        );
        assert_eq!(t.host.tcp_closed, [id]);
    }

    #[test]
    fn setxmpp_xmpp_and_spy_storage() {
        let mut t = server(CONFIG);
        let id = online(&mut t);
        let alice = t.join("Alice");
        t.tick(1);
        sent(&mut t, id);
        assert_eq!(t.cmd(alice, "/setxmpp alice@example.org"), ["XMPP address set to alice@example.org"]);
        let users = t.ess.read_yaml("xmpp/users.yml");
        assert_eq!(users["alice"]["address"], "alice@example.org");
        assert_eq!(users["alice"]["spy"], false);
        assert_eq!(t.cmd(alice, "/xmpp Alice hello <b>there"), ["[Alice>alice@example.org] hello <b>there"]);
        assert_eq!(sent(&mut t, id), "<message to='alice@example.org' type='chat'><body>[Alice] hello &lt;b&gt;there</body></message>");
        assert_eq!(t.cmd(alice, "/xmpp nobody hi"), ["There are no players matching that name."]);
        assert_eq!(t.cmd(alice, "/xmpp alice").len(), 2, "usage: description and usage lines");
        assert_eq!(t.console("xmpp alice from console"), ["[Console>alice@example.org] from console"]);
        assert_eq!(sent(&mut t, id), "<message to='alice@example.org' type='chat'><body>[Console] from console</body></message>");
        assert_eq!(t.console("setxmpp a@b"), ["Error: Only in-game players can use /setxmpp."]);

        // xmppspy toggles and keeps the address.
        assert_eq!(t.console("xmppspy Alice"), ["XMPP Spy enabled for Alice"]);
        assert_eq!(t.ess.read_yaml("xmpp/users.yml")["alice"], json!({"address": "alice@example.org", "spy": true}));
        assert_eq!(spy_users(&t.ess), ["alice@example.org"]);
        assert_eq!(t.console("xmppspy *"), ["XMPP Spy disabled for Alice"]);
        assert!(spy_users(&t.ess).is_empty());
        assert_eq!(t.console("xmppspy nobody"), ["Error: Player not found."]);
        // A spy without an address is stored but relays nowhere.
        let bob = t.join("Bob");
        t.op("Bob");
        assert_eq!(t.cmd(bob, "/xmppspy @s"), ["XMPP Spy enabled for Bob"]);
        assert_eq!(t.ess.read_yaml("xmpp/users.yml")["bob"]["spy"], true);
        assert!(spy_users(&t.ess).is_empty());
    }

    #[test]
    fn permissions_follow_the_nodes() {
        let mut t = server(CONFIG);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.setxmpp]\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/xmpp x y"), ["You do not have access to that command."]);
        assert_eq!(t.cmd(bob, "/xmppspy Bob"), ["You do not have access to that command."]);
        assert_eq!(t.cmd(bob, "/setxmpp b@x"), ["XMPP address set to b@x"]);
    }

    #[test]
    fn spies_get_joins_quits_and_chat_and_presence_follows_the_count() {
        let mut t = server(CONFIG);
        let id = online(&mut t);
        std::fs::write(t.ess.dir.join("xmpp/users.yml"), "spyguy:\n  address: spy@example.org\n  spy: true\n").unwrap();
        let alice = t.join("Alice");
        t.tick(1);
        assert_eq!(
            sent(&mut t, id),
            "<presence><status>1 player online.</status><priority>2</priority></presence><message to='spy@example.org' type='chat'><body>Player Alice joined the game</body></message>"
        );
        assert_eq!(t.chat(alice, "hi all"), Chat::Pass);
        t.tick(1);
        assert_eq!(sent(&mut t, id), "<message to='spy@example.org' type='chat'><body>&lt;Alice&gt; hi all</body></message>");
        t.join("Bob");
        t.tick(1);
        assert!(sent(&mut t, id).starts_with("<presence><status>2 players online.</status>"));
        t.quit(alice);
        t.tick(1);
        let out = sent(&mut t, id);
        assert!(out.contains("Player Alice left the game"), "{out}");
    }

    #[test]
    fn incoming_messages_chat_whisper_and_commands() {
        let mut t = server(CONFIG);
        let id = online(&mut t);
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        t.cmd(alice, "/setxmpp alice@example.org");
        std::fs::write(
            t.ess.dir.join("xmpp/users.yml"),
            "alice:\n  address: alice@example.org\n  spy: true\nspyguy:\n  address: spy@example.org\n  spy: true\n",
        )
        .unwrap();
        t.tick(1);
        sent(&mut t, id);
        // Chat from a linked address: everyone, and the other spies.
        feed(&mut t, id, "<message from='alice@example.org/phone' type='chat'><body>hello &amp; bye</body></message>");
        assert_eq!(t.said(bob), ["=Alice: hello & bye"]);
        assert_eq!(t.said(alice), ["=Alice: hello & bye"]);
        assert_eq!(sent(&mut t, id), "<message to='spy@example.org' type='chat'><body>=Alice: hello &amp; bye</body></message>");
        // Unlinked addresses and errors are dropped.
        feed(&mut t, id, "<message from='who@x.org' type='chat'><body>hey</body></message>");
        feed(&mut t, id, "<message from='alice@example.org' type='error'><body>oops</body></message>");
        assert!(t.said(bob).is_empty());
        // @player whispers.
        feed(&mut t, id, "<message from='alice@example.org/phone' type='chat'><body>@bo psst</body></message>");
        assert_eq!(t.said(bob), ["[alice>Bob]  @bo psst"]);
        feed(&mut t, id, "<message from='alice@example.org/phone' type='chat'><body>@zed psst</body></message>");
        assert_eq!(sent(&mut t, id), "<message to='alice@example.org' type='chat'><body>User @zed not found</body></message>");
        // Commands only from op-users, run as the console.
        feed(&mut t, id, "<message from='alice@example.org' type='chat'><body>/time set day</body></message>");
        t.tick(1);
        assert!(t.host.console_commands.is_empty());
        feed(&mut t, id, "<message from='admin@example.org/pc' type='chat'><body>/vanillacmd arg</body></message>");
        t.tick(1);
        assert_eq!(t.host.console_commands, ["vanillacmd arg"]);
        feed(&mut t, id, "<message from='admin@example.org/pc' type='chat'><body>/ping</body></message>");
        t.tick(1);
        assert!(t.host.said(Sender::Console).contains(&"Pong!".to_string()));
    }

    #[test]
    fn server_pings_subscriptions_and_disconnects() {
        let mut t = server(CONFIG);
        let id = online(&mut t);
        feed(&mut t, id, "<iq type='get' id='p1' from='chat.example.org'><ping xmlns='urn:xmpp:ping'/></iq>");
        assert_eq!(sent(&mut t, id), "<iq type='result' id='p1' to='chat.example.org'/>");
        feed(&mut t, id, "<iq type='get' id='v1' from='x@y'><query xmlns='jabber:iq:version'/></iq>");
        assert!(sent(&mut t, id).contains("service-unavailable"));
        feed(&mut t, id, "<presence from='new@example.org' type='subscribe'/>");
        assert_eq!(sent(&mut t, id), "<presence to='new@example.org' type='subscribed'/>");
        // The socket drops: logged, reconnect later.
        t.host.said(Sender::Console);
        t.host.tcp_incoming.push((id, Vec::new()));
        t.tick(1);
        assert_eq!(t.host.said(Sender::Console), ["Disconnected from the XMPP server."]);
        assert_eq!(t.host.tcp_opened.len(), 1);
        t.tick(RECONNECT_TICKS as u32);
        assert_eq!(t.host.tcp_opened.len(), 2);
    }

    #[test]
    fn xml_and_base64_helpers() {
        assert_eq!(base64(b"\0bot\0secret"), "AGJvdABzZWNyZXQ=");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        let (ev, used) = next_event(b"  <a x=\"1\" y='&lt;'><b/>t&#65;<![CDATA[<c>]]><!-- no --></a>rest").unwrap();
        assert_eq!(used, 60);
        let Event::Stanza(el) = ev else { panic!() };
        assert_eq!(el.attr("y"), Some("<"));
        assert_eq!(el.child("b").map(|b| b.name.as_str()), Some("b"));
        assert_eq!(el.text, "tA<c>");
        assert!(next_event(b"<a><b>").is_none());
        assert_eq!(next_event(b"</stream:stream>").unwrap().0, Event::StreamClose);
        // A multi-byte character split across reads waits for the rest.
        let msg = "<m>é</m>".as_bytes();
        assert!(next_event(&msg[..4]).is_none());
        assert!(next_event(msg).is_some());
    }
}
