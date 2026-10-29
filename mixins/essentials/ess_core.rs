// horizon-essentials: a port of EssentialsX (GPL-3.0) to Horizon. Pure logic, no Horizon types: this file is
// the injected module `crate::game::ess_core` and, unchanged, part of crates/ess-standalone (fast tests).
//
// The shape follows EssentialsX: `Ess` is the plugin (settings, users, permissions, scheduler, commands);
// `Host` is everything Essentials asks of the server (Bukkit's role), implemented over Horizon's Game/Hub by
// ess_glue and by `ess_testkit::FakeHost` in tests. Feature areas live in their own `ess_<area>` modules and
// register commands and listeners in `register`; ess_areas lists them.

use crate::game::{ess_meta, ess_messages, ess_text, ess_yaml};
use serde_json::{Value, json};
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub use crate::game::ess_text::{Arg, Span};

pub type Uuid = u128;

/// `8-4-4-4-12` lower-case hex, as Bukkit prints a UUID (userdata file names).
pub fn uuid_string(u: Uuid) -> String {
    let h = format!("{u:032x}");
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

pub fn parse_uuid(s: &str) -> Option<Uuid> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    (hex.len() == 32).then(|| u128::from_str_radix(&hex, 16).ok()).flatten()
}

/// `/ping [text]`: "Pong!", or the text echoed back (Commandping).
pub fn ping(args: &str) -> String {
    let text = args.trim();
    if text.is_empty() { "Pong!".to_string() } else { text.to_string() }
}

// ===== World model shared by every area ==========================================================

/// A position in a world (dimension id such as `minecraft:overworld`), with a facing.
#[derive(Clone, Debug, PartialEq)]
pub struct Loc {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

impl Loc {
    pub fn new(world: &str, x: f64, y: f64, z: f64) -> Self {
        Self { world: world.to_string(), x, y, z, yaw: 0.0, pitch: 0.0 }
    }
    pub fn block(&self) -> [i32; 3] {
        [self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32]
    }
    pub fn distance(&self, other: &Loc) -> f64 {
        if self.world != other.world {
            return f64::INFINITY;
        }
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }
    /// As EssentialsX stores a location in YAML (world, x, y, z, yaw, pitch).
    pub fn to_yaml(&self) -> Value {
        json!({"world": self.world, "x": self.x, "y": self.y, "z": self.z, "yaw": self.yaw, "pitch": self.pitch})
    }
    pub fn from_yaml(v: &Value) -> Option<Loc> {
        let f = |k: &str| v.get(k).and_then(Value::as_f64);
        Some(Loc {
            world: v.get("world").and_then(Value::as_str).map(world_id)?,
            x: f("x")?,
            y: f("y")?,
            z: f("z")?,
            yaw: f("yaw").unwrap_or(0.0) as f32,
            pitch: f("pitch").unwrap_or(0.0) as f32,
        })
    }
}

/// Bukkit world names (`world`, `world_nether`, `world_the_end`) as Horizon's dimension ids; ids pass through.
pub fn world_id(name: &str) -> String {
    match name {
        "world" | "overworld" => "minecraft:overworld".into(),
        "world_nether" | "nether" | "the_nether" => "minecraft:the_nether".into(),
        "world_the_end" | "end" | "the_end" => "minecraft:the_end".into(),
        other if other.contains(':') => other.into(),
        other => format!("minecraft:{other}"),
    }
}

/// An item stack: the item id and its vanilla data components (as `/give`'s JSON-ish form: `custom_name`,
/// `lore`, `enchantments`, `damage`...). Empty slots are `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: String,
    pub count: i32,
    pub components: Value,
}

impl Item {
    pub fn new(id: &str, count: i32) -> Self {
        let id = if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") };
        Self { id, count, components: json!({}) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "0" | "s" | "survival" | "survi" | "surv" => Some(Self::Survival),
            "1" | "c" | "creative" | "creat" => Some(Self::Creative),
            "2" | "a" | "adventure" | "adven" => Some(Self::Adventure),
            "3" | "sp" | "spec" | "spectator" | "spect" => Some(Self::Spectator),
            _ => None,
        }
    }
}

/// Who runs a command or receives a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sender {
    Console,
    Player(Uuid),
}

impl Sender {
    pub fn player(self) -> Option<Uuid> {
        match self {
            Sender::Player(u) => Some(u),
            Sender::Console => None,
        }
    }
}

/// A container a plugin opens for a player (invsee, enderchest of others, kit preview, disposal...).
#[derive(Clone, Debug, PartialEq)]
pub struct Container {
    /// Rows of 9 (1..=6).
    pub rows: u8,
    /// MiniMessage title.
    pub title: String,
    pub items: Vec<Option<Item>>,
    /// An id the opening area chooses, handed back on clicks and close.
    pub tag: String,
}

/// A vanilla workstation opened without a block (/workbench, /anvil...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    Crafting,
    Anvil,
    Cartography,
    Grindstone,
    Loom,
    Smithing,
    Stonecutter,
    EnderChest,
}

/// One entry of the server's ban lists.
#[derive(Clone, Debug, PartialEq)]
pub struct Ban {
    pub target: String,
    pub reason: String,
    pub source: String,
    /// Unix millis; None = permanent.
    pub expires: Option<i64>,
}

/// What the server can say about itself (/gc, /essentials).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServerStats {
    pub tps: f64,
    pub mspt: f64,
    pub uptime_ms: i64,
    pub memory_used_mb: u64,
    pub memory_max_mb: u64,
    /// (world, loaded chunks, entities, tile entities)
    pub worlds: Vec<(String, u32, u32, u32)>,
}

/// Everything Essentials asks of the server. Methods default to "not supported" so a host may grow
/// method by method; every default is safe (no change, empty answer).
#[allow(unused_variables)]
pub trait Host {
    // ----- Time ---------------------------------------------------------------------------------
    /// Wall clock, Unix millis (timestamps, cooldowns, tempbans).
    fn now_millis(&mut self) -> i64;

    // ----- Players --------------------------------------------------------------------------------
    fn online(&mut self) -> Vec<Uuid>;
    fn name(&mut self, player: Uuid) -> Option<String>;
    /// A known player (online or in the user cache) by name, any case.
    fn lookup(&mut self, name: &str) -> Option<(Uuid, String)> {
        None
    }
    /// The vanilla operator level (0 = not an operator).
    fn op_level(&mut self, player: Uuid) -> u8 {
        0
    }
    fn address(&mut self, player: Uuid) -> Option<String> {
        None
    }
    fn kick(&mut self, player: Uuid, reason: &[Span]) {}

    // ----- Messages -------------------------------------------------------------------------------
    /// A chat line to a player (system chat) or the console (log).
    fn send(&mut self, to: Sender, text: &[Span]);
    fn action_bar(&mut self, player: Uuid, text: &[Span]) {}
    /// A line to everyone online and the console.
    fn broadcast(&mut self, text: &[Span]) {
        for p in self.online() {
            self.send(Sender::Player(p), text);
        }
        self.send(Sender::Console, text);
    }
    /// Text sent as the player's own chat, unsigned (chat formats, /me, sudo c:).
    fn chat_as(&mut self, player: Uuid, text: &[Span]) {
        self.broadcast(text);
    }

    // ----- Places ---------------------------------------------------------------------------------
    fn location(&mut self, player: Uuid) -> Option<Loc>;
    /// Moves a player, across dimensions too. False when it can't.
    fn teleport(&mut self, player: Uuid, to: &Loc) -> bool {
        false
    }
    fn worlds(&mut self) -> Vec<String> {
        vec!["minecraft:overworld".into(), "minecraft:the_nether".into(), "minecraft:the_end".into()]
    }
    fn world_spawn(&mut self, world: &str) -> Option<Loc> {
        None
    }
    fn set_world_spawn(&mut self, at: &Loc) {}
    /// The player's respawn point (bed or anchor), if set.
    fn bed_spawn(&mut self, player: Uuid) -> Option<Loc> {
        None
    }
    /// A block's state, `minecraft:stone` or `minecraft:oak_log[axis=y]`; None in unloaded chunks.
    fn block(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        None
    }
    fn set_block(&mut self, world: &str, pos: [i32; 3], state: &str) -> bool {
        false
    }
    /// The y of the highest non-air block at x, z (Bukkit getHighestBlockYAt).
    fn highest_y(&mut self, world: &str, x: i32, z: i32) -> Option<i32> {
        None
    }
    /// The world's height range (min_y, max_y exclusive).
    fn height_range(&mut self, world: &str) -> (i32, i32) {
        (-64, 320)
    }
    /// The block the player looks at, within `max` blocks.
    fn target_block(&mut self, player: Uuid, max: f64) -> Option<[i32; 3]> {
        None
    }
    fn biome(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        None
    }
    /// A sign's four front lines (MiniMessage/legacy text), or None if no sign is there.
    fn sign_lines(&mut self, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
        None
    }
    fn set_sign_lines(&mut self, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
        false
    }

    // ----- Player state ---------------------------------------------------------------------------
    fn game_mode(&mut self, player: Uuid) -> GameMode {
        GameMode::Survival
    }
    fn set_game_mode(&mut self, player: Uuid, mode: GameMode) {}
    /// (health, max health)
    fn health(&mut self, player: Uuid) -> (f64, f64) {
        (20.0, 20.0)
    }
    fn set_health(&mut self, player: Uuid, health: f64) {}
    /// (food, saturation)
    fn food(&mut self, player: Uuid) -> (i32, f32) {
        (20, 5.0)
    }
    fn set_food(&mut self, player: Uuid, food: i32, saturation: f32) {}
    fn set_fire_ticks(&mut self, player: Uuid, ticks: i32) {}
    fn fire_ticks(&mut self, player: Uuid) -> i32 {
        0
    }
    fn set_freeze_ticks(&mut self, player: Uuid, ticks: i32) {}
    /// Total experience points.
    fn exp(&mut self, player: Uuid) -> i64 {
        0
    }
    fn set_exp(&mut self, player: Uuid, total: i64) {}
    fn flying(&mut self, player: Uuid) -> bool {
        false
    }
    fn set_allow_flight(&mut self, player: Uuid, allow: bool) {}
    fn allow_flight(&mut self, player: Uuid) -> bool {
        false
    }
    fn set_flying(&mut self, player: Uuid, flying: bool) {}
    /// Bukkit speeds: -1.0..=1.0 (defaults 0.1 fly, 0.2 walk).
    fn set_fly_speed(&mut self, player: Uuid, speed: f32) {}
    fn set_walk_speed(&mut self, player: Uuid, speed: f32) {}
    fn set_invulnerable(&mut self, player: Uuid, invulnerable: bool) {}
    /// Hidden from other players (vanish); `seers` may still see them.
    fn set_hidden(&mut self, player: Uuid, hidden: bool, seers: &[Uuid]) {}
    /// Name shown in chat and the tab list (MiniMessage), None to reset (/nick, afk list name).
    fn set_display_name(&mut self, player: Uuid, name: Option<&[Span]>) {}
    fn set_tab_name(&mut self, player: Uuid, name: Option<&[Span]>) {}
    /// Per-player time (/ptime): Some((ticks, relative)) or None to follow the world.
    fn set_player_time(&mut self, player: Uuid, time: Option<(i64, bool)>) {}
    /// Per-player weather (/pweather): Some(true) = downfall, Some(false) = clear, None = the world's.
    fn set_player_weather(&mut self, player: Uuid, downfall: Option<bool>) {}
    /// A vanilla statistic (`minecraft:custom/minecraft:play_time`).
    fn statistic(&mut self, player: Uuid, stat: &str) -> i64 {
        0
    }

    // ----- Inventories ---------------------------------------------------------------------------
    /// 41 slots as Bukkit numbers them: 0-8 hotbar, 9-35 main, 36-39 boots..helmet, 40 offhand.
    fn inventory(&mut self, player: Uuid) -> Vec<Option<Item>> {
        vec![None; 41]
    }
    fn set_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {}
    /// The selected hotbar slot (0-8).
    fn held_slot(&mut self, player: Uuid) -> usize {
        0
    }
    fn ender_chest(&mut self, player: Uuid) -> Vec<Option<Item>> {
        vec![None; 27]
    }
    fn set_ender_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {}
    /// Opens a plugin container for the player.
    fn open_container(&mut self, player: Uuid, container: &Container) -> bool {
        false
    }
    fn open_station(&mut self, player: Uuid, station: Station) -> bool {
        false
    }
    fn close_container(&mut self, player: Uuid) {}
    /// Drops an item at a location (overflowing kits, /give when full).
    fn drop_item(&mut self, at: &Loc, item: &Item) {}

    // ----- Registries ------------------------------------------------------------------------------
    /// Every item id (`minecraft:stone`...), for /itemdb, item names and validation.
    fn item_ids(&mut self) -> Vec<String> {
        Vec::new()
    }
    fn max_stack(&mut self, item: &str) -> i32 {
        64
    }
    /// Maximum durability, 0 for items without damage.
    fn max_damage(&mut self, item: &str) -> i32 {
        0
    }
    /// Every enchantment id with its max level.
    fn enchantments(&mut self) -> Vec<(String, i32)> {
        Vec::new()
    }
    fn entity_types(&mut self) -> Vec<String> {
        Vec::new()
    }

    // ----- World ----------------------------------------------------------------------------------
    fn world_time(&mut self, world: &str) -> i64 {
        0
    }
    fn set_world_time(&mut self, world: &str, time: i64) {}
    /// (raining, thundering)
    fn weather(&mut self, world: &str) -> (bool, bool) {
        (false, false)
    }
    /// Sets weather; duration in ticks (None = vanilla random).
    fn set_weather(&mut self, world: &str, rain: bool, thunder: bool, duration: Option<i32>) {}
    /// Spawns an entity (`minecraft:zombie`) with optional SNBT data; returns whether it spawned.
    fn spawn_entity(&mut self, kind: &str, at: &Loc, snbt: Option<&str>) -> bool {
        false
    }
    fn strike_lightning(&mut self, at: &Loc, effect_only: bool) {}
    /// Removes entities of the given kinds within `radius` (None = whole world); returns how many.
    fn remove_entities(&mut self, world: &str, center: Option<&Loc>, radius: Option<f64>, kinds: &[String]) -> usize {
        0
    }
    /// Players within `radius` of a location (excluding none).
    fn players_near(&mut self, at: &Loc, radius: f64) -> Vec<Uuid> {
        let mut out = Vec::new();
        for p in self.online() {
            if self.location(p).is_some_and(|l| l.distance(at) <= radius) {
                out.push(p);
            }
        }
        out
    }

    // ----- Server ---------------------------------------------------------------------------------
    /// A vanilla command run by the console (level 4). Returns (success, feedback lines).
    fn console_command(&mut self, line: &str) -> (bool, Vec<String>) {
        (false, Vec::new())
    }
    /// A vanilla command run as a player, with that player's permissions (or forced level 4 when `op`).
    fn player_command(&mut self, player: Uuid, line: &str, op: bool) -> (bool, Vec<String>) {
        (false, Vec::new())
    }
    fn stats(&mut self) -> ServerStats {
        ServerStats::default()
    }
    fn max_players(&mut self) -> usize {
        20
    }
    fn set_motd(&mut self, motd: &[Span]) {}
    fn bans(&mut self, ip: bool) -> Vec<Ban> {
        Vec::new()
    }
    fn ban(&mut self, ip: bool, ban: &Ban) {}
    fn unban(&mut self, ip: bool, target: &str) -> bool {
        false
    }
    /// The server folder; Essentials' data lives in `<server>/essentials`.
    fn server_dir(&mut self) -> PathBuf {
        PathBuf::from(".")
    }

    // ----- Area extensions ------------------------------------------------------------------------
    // An area that needs more of the server adds methods in ITS OWN region below, each with a safe default
    // body, then overrides it in ess_testkit's FakeHost region and in ess_glue's region (same area name).
    // ---- area tp ----
    // ---- end tp ----
    // ---- area social ----
    // ---- end social ----
    // ---- area admin ----
    // ---- end admin ----
    // ---- area items ----
    // ---- end items ----
    // ---- area economy ----
    /// Whether the player is sneaking (bulk buying and selling on [Buy]/[Sell] signs).
    fn sneaking(&mut self, player: Uuid) -> bool {
        false
    }
    /// Whether an item id places a block (`/sell blocks`, `/repair` skips blocks; Material.isBlock).
    fn item_is_block(&mut self, item: &str) -> bool {
        false
    }
    // ---- end economy ----
    // ---- area world ----
    // ---- end world ----
}

// ===== Events =====================================================================================

/// What a chat listener decides about a player's chat message.
#[derive(Clone, Debug, PartialEq)]
pub enum Chat {
    /// Let it through unchanged (signed, as vanilla).
    Pass,
    /// Drop it.
    Cancel,
    /// Drop the original and show these lines instead (unsigned): (recipients, text). Recipients None = everyone.
    Replace(Vec<(Option<Vec<Uuid>>, Vec<Span>)>),
}

/// Listener results that may stop the action (block break, damage...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Allow {
    Yes,
    No,
}

pub type JoinFn = fn(&mut Ess, &mut dyn Host, Uuid, bool);
pub type PlayerFn = fn(&mut Ess, &mut dyn Host, Uuid);
pub type TickFn = fn(&mut Ess, &mut dyn Host);
pub type ChatFn = fn(&mut Ess, &mut dyn Host, Uuid, &str) -> Chat;
pub type MoveFn = fn(&mut Ess, &mut dyn Host, Uuid, &Loc, &Loc);
pub type DeathFn = fn(&mut Ess, &mut dyn Host, Uuid, &Loc);
pub type RespawnFn = fn(&mut Ess, &mut dyn Host, Uuid) -> Option<Loc>;
pub type DamageFn = fn(&mut Ess, &mut dyn Host, Uuid, &str, f64) -> Allow;
pub type BlockFn = fn(&mut Ess, &mut dyn Host, Uuid, &str, [i32; 3], &str) -> Allow;
pub type SignFn = fn(&mut Ess, &mut dyn Host, Uuid, &str, [i32; 3], &mut [String; 4]) -> Allow;
pub type InteractFn = fn(&mut Ess, &mut dyn Host, Uuid, Option<(&str, [i32; 3], &str)>, Option<&Item>) -> Allow;
pub type ClickFn = fn(&mut Ess, &mut dyn Host, Uuid, &str, usize) -> Allow;
pub type CloseFn = fn(&mut Ess, &mut dyn Host, Uuid, &str, &[Option<Item>]);
pub type WorldFn = fn(&mut Ess, &mut dyn Host, &str, &str, [i32; 3]) -> Allow;
pub type CommandHookFn = fn(&mut Ess, &mut dyn Host, Sender, &str) -> Allow;

/// Every event Essentials listens to. Areas push their functions in `register`.
#[derive(Default)]
pub struct Listeners {
    /// A player joined (true on the first join ever).
    pub join: Vec<JoinFn>,
    pub quit: Vec<PlayerFn>,
    /// Once per server tick (20 a second).
    pub tick: Vec<TickFn>,
    /// A player's chat message, before it is broadcast. The first non-Pass answer wins.
    pub chat: Vec<ChatFn>,
    /// Any command line a player or the console sends, before dispatch (jail, mute, cooldowns, spy, costs).
    pub command: Vec<CommandHookFn>,
    /// A player moved (from, to); checked at most once a tick.
    pub moved: Vec<MoveFn>,
    pub death: Vec<DeathFn>,
    /// Where a player respawns; the first Some wins over the vanilla respawn point.
    pub respawn: Vec<RespawnFn>,
    /// Damage to a player (cause, amount).
    pub damage: Vec<DamageFn>,
    /// (player, world, pos, block state) about to be broken / placed.
    pub block_break: Vec<BlockFn>,
    pub block_place: Vec<BlockFn>,
    /// A player finished editing a sign: (world, pos, lines), lines may be rewritten.
    pub sign_change: Vec<SignFn>,
    /// A right click: on a block (world, pos, state) or in the air, with the held item.
    pub interact: Vec<InteractFn>,
    /// A click in a plugin container (tag, raw slot); No cancels it.
    pub container_click: Vec<ClickFn>,
    pub container_close: Vec<CloseFn>,
    /// World changes not caused by a player: (kind: "explode"|"fire_spread"|"burn"|"flow"|"spawn:<entity>", world, pos).
    pub world: Vec<WorldFn>,
}

// ===== Commands ===================================================================================

/// Why a command stopped (EssentialsX's exceptions).
#[derive(Clone, Debug, PartialEq)]
pub enum CmdError {
    /// NotEnoughArgumentsException: show the usage (with an optional extra MiniMessage line).
    Usage(Option<String>),
    /// PlayerNotFoundException.
    PlayerNotFound,
    /// NoChargeException / QuietAbortException: stop silently.
    Quiet,
    /// TranslatableException: `errorWithMessage` around a translated key.
    Tl(&'static str, Vec<Arg>),
    /// A plain exception message (errorWithMessage).
    Msg(String),
}

pub type CmdResult = Result<(), CmdError>;
pub type CmdFn = fn(&mut Ctx, &[String]) -> CmdResult;
pub type TabFn = fn(&mut Ctx, &[String]) -> Vec<String>;

/// One command implementation, keyed by its plugin.yml name (aliases come from ess_meta).
#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub run: CmdFn,
    /// Suggestions for the last argument (optional).
    pub tab: Option<TabFn>,
    /// Usage lines for verbose-command-usages: (syntax, translation key of its description).
    pub usages: &'static [(&'static str, &'static str)],
}

/// The context of one command run.
pub struct Ctx<'a> {
    pub ess: &'a mut Ess,
    pub host: &'a mut dyn Host,
    pub sender: Sender,
    /// The label typed (`homes`, `ehome`...).
    pub label: String,
    /// The plugin.yml command name (`home`).
    pub name: &'static str,
}

impl Ctx<'_> {
    /// The sender as a player, or the "only players" error.
    pub fn player(&self) -> Result<Uuid, CmdError> {
        self.sender.player().ok_or(CmdError::Tl("onlyPlayers", vec![Arg::Plain(format!("/{}", self.label))]))
    }
    pub fn is_player(&self) -> bool {
        self.sender.player().is_some()
    }
    pub fn has(&mut self, node: &str) -> bool {
        self.ess.has(self.host, self.sender, node)
    }
    /// Sends a translated message to the sender.
    pub fn tl(&mut self, key: &str, args: &[Arg]) {
        let to = self.sender;
        self.ess.send_tl(self.host, to, key, args);
    }
    pub fn tl_to(&mut self, to: Sender, key: &str, args: &[Arg]) {
        self.ess.send_tl(self.host, to, key, args);
    }
    pub fn send_mini(&mut self, mini: &str) {
        let to = self.sender;
        self.ess.send_mini(self.host, to, mini);
    }
    /// Essentials' matchUser: an online player by exact name, then by name prefix (hidden players excluded
    /// for players who can't see them), else PlayerNotFound.
    pub fn online_player(&mut self, search: &str) -> Result<Uuid, CmdError> {
        self.ess.match_player(self.host, self.sender, search, false).ok_or(CmdError::PlayerNotFound)
    }
    /// Like online_player, but known offline players match too.
    pub fn any_player(&mut self, search: &str) -> Result<Uuid, CmdError> {
        self.ess.match_player(self.host, self.sender, search, true).ok_or(CmdError::PlayerNotFound)
    }
    pub fn name_of(&mut self, player: Uuid) -> String {
        self.host.name(player).or_else(|| self.ess.users.get(player).and_then(|u| u.name())).unwrap_or_else(|| uuid_string(player))
    }
}

/// The joined arguments from `from` (EssentialsCommand.getFinalArg).
pub fn final_arg(args: &[String], from: usize) -> String {
    args.get(from..).map(|a| a.join(" ")).unwrap_or_default()
}

// ===== Users ======================================================================================

/// One player's Essentials data (userdata/<uuid>.yml), kept as YAML so every area adds its own keys.
#[derive(Clone, Debug, Default)]
pub struct User {
    pub uuid: Uuid,
    pub data: Value,
    pub dirty: bool,
}

impl User {
    pub fn get(&self, path: &str) -> Option<&Value> {
        ess_yaml::get(&self.data, path)
    }
    pub fn set(&mut self, path: &str, value: Value) {
        ess_yaml::set(&mut self.data, path, value);
        self.dirty = true;
    }
    pub fn str(&self, path: &str) -> Option<String> {
        self.get(path).and_then(Value::as_str).map(str::to_string)
    }
    pub fn bool(&self, path: &str) -> bool {
        self.get(path).and_then(Value::as_bool).unwrap_or(false)
    }
    pub fn i64(&self, path: &str) -> Option<i64> {
        self.get(path).and_then(Value::as_i64)
    }
    pub fn f64(&self, path: &str) -> Option<f64> {
        self.get(path).and_then(Value::as_f64)
    }
    pub fn name(&self) -> Option<String> {
        self.str("last-account-name")
    }
}

/// Every user Essentials knows, loaded lazily from `essentials/userdata`.
#[derive(Default)]
pub struct Users {
    pub dir: PathBuf,
    pub loaded: HashMap<Uuid, User>,
    /// Lower-case name -> uuid (usermap), from userdata files and joins.
    pub names: BTreeMap<String, Uuid>,
}

impl Users {
    fn file(&self, uuid: Uuid) -> PathBuf {
        self.dir.join(format!("{}.yml", uuid_string(uuid)))
    }
    pub fn exists(&self, uuid: Uuid) -> bool {
        self.loaded.contains_key(&uuid) || self.file(uuid).exists()
    }
    /// The user if Essentials has data for them (no new file).
    pub fn get(&mut self, uuid: Uuid) -> Option<&mut User> {
        if !self.loaded.contains_key(&uuid) {
            let text = std::fs::read_to_string(self.file(uuid)).ok()?;
            let data = ess_yaml::parse(&text).unwrap_or(json!({}));
            self.loaded.insert(uuid, User { uuid, data, dirty: false });
        }
        self.loaded.get_mut(&uuid)
    }
    /// The user, created if new.
    pub fn user(&mut self, uuid: Uuid) -> &mut User {
        if self.get(uuid).is_none() {
            self.loaded.insert(uuid, User { uuid, data: json!({}), dirty: true });
        }
        self.loaded.get_mut(&uuid).expect("just inserted")
    }
    /// Every uuid with a userdata file or in memory.
    pub fn all(&self) -> Vec<Uuid> {
        let mut out: Vec<Uuid> = self.loaded.keys().copied().collect();
        if let Ok(dir) = std::fs::read_dir(&self.dir) {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if let Some(u) = name.strip_suffix(".yml").and_then(parse_uuid) {
                    if !out.contains(&u) {
                        out.push(u);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }
    pub fn save(&mut self) {
        let _ = std::fs::create_dir_all(&self.dir);
        let dir = self.dir.clone();
        for user in self.loaded.values_mut().filter(|u| u.dirty) {
            let path = dir.join(format!("{}.yml", uuid_string(user.uuid)));
            let tmp = path.with_extension("yml.tmp");
            if std::fs::write(&tmp, ess_yaml::to_string(&user.data)).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
                user.dirty = false;
            }
        }
    }
    fn index_names(&mut self) {
        for uuid in self.all() {
            if let Some(name) = self.get(uuid).and_then(|u| u.name()) {
                self.names.insert(name.to_lowercase(), uuid);
            }
        }
    }
}

// ===== Permissions ================================================================================

/// Essentials' permission checks without a permissions plugin: `essentials/permissions.yml` groups
/// (with inheritance, wildcards and `-negation`), per-user nodes, then plugin.yml defaults by op status.
#[derive(Default)]
pub struct Perms {
    /// group -> (nodes, inherited groups, prefix, suffix)
    pub groups: BTreeMap<String, Group>,
    /// lower-case name or uuid -> (groups, nodes)
    pub users: BTreeMap<String, (Vec<String>, Vec<String>)>,
    pub default_group: String,
}

#[derive(Clone, Debug, Default)]
pub struct Group {
    pub nodes: Vec<String>,
    pub inherits: Vec<String>,
    pub prefix: String,
    pub suffix: String,
}

impl Perms {
    pub fn load(yaml: &Value) -> Perms {
        let list = |v: Option<&Value>| -> Vec<String> {
            match v {
                Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_lowercase)).collect(),
                Some(Value::String(s)) => vec![s.to_lowercase()],
                _ => Vec::new(),
            }
        };
        let mut perms = Perms { default_group: "default".into(), ..Perms::default() };
        if let Some(Value::Object(groups)) = yaml.get("groups") {
            for (name, g) in groups {
                if g.get("default").and_then(Value::as_bool) == Some(true) {
                    perms.default_group = name.to_lowercase();
                }
                perms.groups.insert(
                    name.to_lowercase(),
                    Group {
                        nodes: list(g.get("permissions")),
                        inherits: list(g.get("inherits")),
                        prefix: g.get("prefix").and_then(Value::as_str).unwrap_or("").to_string(),
                        suffix: g.get("suffix").and_then(Value::as_str).unwrap_or("").to_string(),
                    },
                );
            }
        }
        if let Some(Value::Object(users)) = yaml.get("users") {
            for (key, u) in users {
                perms.users.insert(key.to_lowercase(), (list(u.get("groups")), list(u.get("permissions"))));
            }
        }
        perms
    }

    /// The groups of a player, primary first (their own list, else the default group).
    pub fn groups_of(&self, uuid: Uuid, name: &str) -> Vec<String> {
        let entry = self.users.get(&uuid_string(uuid)).or_else(|| self.users.get(&name.to_lowercase()));
        match entry {
            Some((groups, _)) if !groups.is_empty() => groups.clone(),
            _ => vec![self.default_group.clone()],
        }
    }

    /// Nodes from the user and their groups (user first, then groups breadth-first with inheritance).
    fn nodes(&self, uuid: Uuid, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some((_, nodes)) = self.users.get(&uuid_string(uuid)).or_else(|| self.users.get(&name.to_lowercase())) {
            out.extend(nodes.iter().cloned());
        }
        let mut queue = self.groups_of(uuid, name);
        let mut seen = Vec::new();
        while let Some(g) = (!queue.is_empty()).then(|| queue.remove(0)) {
            if seen.contains(&g) {
                continue;
            }
            seen.push(g.clone());
            if let Some(group) = self.groups.get(&g) {
                out.extend(group.nodes.iter().cloned());
                queue.extend(group.inherits.iter().cloned());
            }
        }
        out
    }

    /// Some(granted) when a configured node decides it: exact, then parent wildcards (`a.b.*`, `a.*`, `*`),
    /// a `-node` negation winning at the same level.
    pub fn configured(&self, uuid: Uuid, name: &str, node: &str) -> Option<bool> {
        let nodes = self.nodes(uuid, name);
        let mut candidates = vec![node.to_string()];
        let parts: Vec<&str> = node.split('.').collect();
        for i in (1..parts.len()).rev() {
            candidates.push(format!("{}.*", parts[..i].join(".")));
        }
        candidates.push("*".into());
        for c in candidates {
            if nodes.iter().any(|n| n == &format!("-{c}")) {
                return Some(false);
            }
            if nodes.iter().any(|n| n == &c) {
                return Some(true);
            }
        }
        // A parent node from plugin.yml whose children grant this node (essentials.kits.* style).
        None
    }
}

/// plugin.yml's default for a node, following parents' `children` (default op when unlisted).
fn default_grant(node: &str, op: bool) -> bool {
    let decide = |d: ess_meta::PermDefault| match d {
        ess_meta::PermDefault::Op => op,
        ess_meta::PermDefault::True => true,
        ess_meta::PermDefault::False => false,
        ess_meta::PermDefault::NotOp => !op,
    };
    if let Some(meta) = ess_meta::permission(node) {
        return decide(meta.default);
    }
    // A node granted as a child of a parent that the player has by default (e.g. essentials.signs.use.*).
    for parent in ess_meta::PERMISSIONS {
        if let Some((_, value)) = parent.children.iter().find(|(c, _)| *c == node) {
            if *value && decide(parent.default) {
                return true;
            }
        }
    }
    op
}

// ===== Scheduler ==================================================================================

pub type Task = Box<dyn FnOnce(&mut Ess, &mut dyn Host)>;

#[derive(Default)]
pub struct Scheduler {
    pub tick: u64,
    tasks: Vec<(u64, u64, Task)>,
    next_id: u64,
    cancelled: Vec<u64>,
}

impl Scheduler {
    /// Runs `task` after `ticks` server ticks (0 = next tick). Returns an id for `cancel`.
    pub fn later(&mut self, ticks: u64, task: Task) -> u64 {
        self.next_id += 1;
        self.tasks.push((self.tick + ticks.max(1), self.next_id, task));
        self.next_id
    }
    pub fn cancel(&mut self, id: u64) {
        self.cancelled.push(id);
    }
    pub fn pending(&self, id: u64) -> bool {
        !self.cancelled.contains(&id) && self.tasks.iter().any(|(_, i, _)| *i == id)
    }
    fn due(&mut self) -> Vec<Task> {
        let now = self.tick;
        let cancelled = std::mem::take(&mut self.cancelled);
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.tasks).into_iter().partition(|(at, _, _)| *at <= now);
        self.tasks = keep.into_iter().filter(|(_, id, _)| !cancelled.contains(id)).collect();
        due.into_iter().filter(|(_, id, _)| !cancelled.contains(id)).map(|(_, _, t)| t).collect()
    }
}

// ===== Ess ========================================================================================

/// The running plugin: what EssentialsX's `Essentials` class holds.
pub struct Ess {
    pub dir: PathBuf,
    /// config.yml (EssentialsX's defaults, then the file on disk).
    pub config: Value,
    pub palette: ess_text::Palette,
    pub users: Users,
    pub perms: Perms,
    pub commands: BTreeMap<&'static str, Command>,
    /// label (name, alias, e-prefixed) -> command name.
    pub labels: BTreeMap<String, &'static str>,
    pub listeners: Listeners,
    pub scheduler: Scheduler,
    /// Messages overriding the built-in ones (messages*.properties in the data folder).
    pub messages: BTreeMap<String, String>,
    /// Per-area state, by type (`ess.state::<HomesState>()`).
    state: HashMap<TypeId, Box<dyn Any>>,
    /// Last known location of online players (for `moved`).
    pub last_loc: HashMap<Uuid, Loc>,
}

impl Ess {
    /// Loads (and on first start writes) Essentials' data folder and registers every area.
    pub fn new(dir: &Path) -> Ess {
        let _ = std::fs::create_dir_all(dir.join("userdata"));
        for (name, text) in crate::game::ess_defaults::FILES {
            let path = dir.join(name);
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, text);
            }
        }
        let perms_path = dir.join("permissions.yml");
        if !perms_path.exists() {
            let _ = std::fs::write(&perms_path, DEFAULT_PERMISSIONS);
        }
        let mut ess = Ess {
            dir: dir.to_path_buf(),
            config: json!({}),
            palette: ess_text::Palette::default(),
            users: Users { dir: dir.join("userdata"), ..Users::default() },
            perms: Perms::default(),
            commands: BTreeMap::new(),
            labels: BTreeMap::new(),
            listeners: Listeners::default(),
            scheduler: Scheduler::default(),
            messages: BTreeMap::new(),
            state: HashMap::new(),
            last_loc: HashMap::new(),
        };
        ess.reload();
        ess.users.index_names();
        crate::game::ess_areas::register_all(&mut ess);
        ess.register(Command { name: "ping", run: cmd_ping, tab: None, usages: &[] });
        ess
    }

    /// Re-reads config.yml, permissions.yml and message overrides (/essentials reload).
    pub fn reload(&mut self) {
        let defaults = crate::game::ess_defaults::file("config.yml").and_then(|t| ess_yaml::parse(t).ok()).unwrap_or(json!({}));
        let file = std::fs::read_to_string(self.dir.join("config.yml")).ok().and_then(|t| ess_yaml::parse(&t).ok());
        let mut config = defaults;
        if let Some(Value::Object(file)) = file {
            merge(&mut config, Value::Object(file));
        }
        self.config = config;
        let color = |ess: &Ess, key: &str, default: &str| {
            ess.config_str(&format!("message-colors.{key}")).unwrap_or_else(|| default.to_string())
        };
        self.palette = ess_text::Palette { primary: color(self, "primary", "#ffaa00"), secondary: color(self, "secondary", "#ff5555") };
        let perms = std::fs::read_to_string(self.dir.join("permissions.yml")).ok().and_then(|t| ess_yaml::parse(&t).ok());
        self.perms = Perms::load(&perms.unwrap_or(json!({})));
        self.messages.clear();
        for name in ["messages.properties", "messages_en.properties"] {
            if let Ok(text) = std::fs::read_to_string(self.dir.join(name)) {
                self.messages.extend(properties(&text));
            }
        }
    }

    // ----- Config -------------------------------------------------------------------------------------
    pub fn config(&self, path: &str) -> Option<&Value> {
        ess_yaml::get(&self.config, path)
    }
    pub fn config_str(&self, path: &str) -> Option<String> {
        self.config(path).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
    }
    pub fn config_bool(&self, path: &str, default: bool) -> bool {
        self.config(path).and_then(Value::as_bool).unwrap_or(default)
    }
    pub fn config_f64(&self, path: &str, default: f64) -> f64 {
        self.config(path).and_then(Value::as_f64).unwrap_or(default)
    }
    pub fn config_i64(&self, path: &str, default: i64) -> i64 {
        self.config(path).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(default)
    }
    pub fn config_list(&self, path: &str) -> Vec<String> {
        match self.config(path) {
            Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string).or_else(|| (!v.is_null()).then(|| v.to_string()))).collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => Vec::new(),
        }
    }
    /// A YAML file in the data folder (warps/x.yml, jail.yml...), Null if missing.
    pub fn read_yaml(&self, name: &str) -> Value {
        std::fs::read_to_string(self.dir.join(name)).ok().and_then(|t| ess_yaml::parse(&t).ok()).unwrap_or(Value::Null)
    }
    pub fn write_yaml(&self, name: &str, value: &Value) {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, ess_yaml::to_string(value)).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    // ----- Area state ---------------------------------------------------------------------------------
    /// An area's own state, created with Default on first use.
    pub fn state<T: Default + 'static>(&mut self) -> &mut T {
        self.state.entry(TypeId::of::<T>()).or_insert_with(|| Box::new(T::default())).downcast_mut::<T>().expect("state type")
    }

    // ----- Registration ---------------------------------------------------------------------------------
    /// Adds a command under its plugin.yml name and every alias (with EssentialsX's `e` prefixes).
    pub fn register(&mut self, command: Command) {
        let name = command.name;
        let mut labels = vec![name.to_string(), format!("e{name}")];
        if let Some(meta) = ess_meta::command(name) {
            labels.extend(meta.aliases.iter().map(|a| a.to_string()));
        }
        for label in labels {
            // A real command name keeps its own label over another command's alias.
            if self.labels.get(&label).is_some_and(|owner| *owner == label.as_str()) && label != name {
                continue;
            }
            self.labels.insert(label, name);
        }
        self.commands.insert(name, command);
    }

    /// The command a label runs, honouring `disabled-commands`.
    pub fn command_for(&self, label: &str) -> Option<&'static str> {
        let label = label.to_lowercase();
        let label = label.rsplit(':').next().unwrap_or(&label).to_string();
        self.labels.get(&label).copied()
    }

    // ----- Messages -----------------------------------------------------------------------------------------
    /// The MiniMessage of a key: overrides, then the built-in English, then the root bundle, else the key.
    pub fn message(&self, key: &str) -> String {
        if let Some(m) = self.messages.get(key) {
            return m.clone();
        }
        for locale in ["en", "root"] {
            if let Some(m) = ess_messages::locale(locale).and_then(|t| ess_messages::lookup(t, key)) {
                return m.to_string();
            }
        }
        key.to_string()
    }
    /// I18n.tl: the message with its arguments (MessageFormat), still MiniMessage.
    pub fn tl(&self, key: &str, args: &[Arg]) -> String {
        ess_text::message_format(&self.message(key), args)
    }
    pub fn render(&self, mini: &str) -> Vec<Span> {
        ess_text::parse_mini(mini, &self.palette)
    }
    pub fn send_mini(&self, host: &mut dyn Host, to: Sender, mini: &str) {
        // A message may hold several lines (\n): each is its own chat line, as Bukkit splits them.
        for line in mini.split('\n') {
            host.send(to, &self.render(line));
        }
    }
    pub fn send_tl(&self, host: &mut dyn Host, to: Sender, key: &str, args: &[Arg]) {
        let text = self.tl(key, args);
        if !text.is_empty() {
            self.send_mini(host, to, &text);
        }
    }
    pub fn broadcast_tl(&self, host: &mut dyn Host, key: &str, args: &[Arg]) {
        let text = self.tl(key, args);
        for line in text.split('\n') {
            host.broadcast(&self.render(line));
        }
    }

    // ----- Permissions ----------------------------------------------------------------------------------------
    /// user.isAuthorized(node): the console has everything.
    pub fn has(&mut self, host: &mut dyn Host, who: Sender, node: &str) -> bool {
        let Sender::Player(uuid) = who else { return true };
        let node = node.to_lowercase();
        let name = host.name(uuid).unwrap_or_default();
        if let Some(granted) = self.perms.configured(uuid, &name, &node) {
            return granted;
        }
        default_grant(&node, host.op_level(uuid) > 0)
    }

    /// The primary group (EssentialsX's user.getGroup()).
    pub fn group(&mut self, host: &mut dyn Host, player: Uuid) -> String {
        let name = host.name(player).unwrap_or_default();
        self.perms.groups_of(player, &name).into_iter().next().unwrap_or_else(|| "default".into())
    }

    // ----- Players ----------------------------------------------------------------------------------------------
    /// Essentials.matchUser: exact name, then display/name prefix among online players; offline by exact name.
    pub fn match_player(&mut self, host: &mut dyn Host, viewer: Sender, search: &str, offline: bool) -> Option<Uuid> {
        let search_l = search.to_lowercase();
        if let Some(u) = parse_uuid(search) {
            if host.online().contains(&u) || (offline && self.users.exists(u)) {
                return Some(u);
            }
        }
        let online = host.online();
        let visible: Vec<Uuid> = online.into_iter().filter(|p| self.can_see(host, viewer, *p)).collect();
        for p in &visible {
            if host.name(*p).is_some_and(|n| n.to_lowercase() == search_l) {
                return Some(*p);
            }
        }
        // Nicknames, then name prefixes.
        for p in &visible {
            let nick = self.users.get(*p).and_then(|u| u.str("nickname")).map(|n| ess_text::strip_legacy(&n, true).to_lowercase());
            if nick.is_some_and(|n| n == search_l || n.trim_start_matches('~') == search_l) {
                return Some(*p);
            }
        }
        let prefixed: Vec<Uuid> = visible.iter().copied().filter(|p| host.name(*p).is_some_and(|n| n.to_lowercase().starts_with(&search_l))).collect();
        if prefixed.len() == 1 {
            return Some(prefixed[0]);
        }
        if offline {
            if let Some(u) = self.users.names.get(&search_l) {
                return Some(*u);
            }
            if let Some((u, name)) = host.lookup(search) {
                self.users.names.insert(name.to_lowercase(), u);
                return Some(u);
            }
        }
        None
    }

    /// Whether `viewer` sees `target` (vanish): hidden players are seen by themselves and `essentials.vanish.see`.
    pub fn can_see(&mut self, host: &mut dyn Host, viewer: Sender, target: Uuid) -> bool {
        if viewer == Sender::Player(target) || !self.is_vanished(target) {
            return true;
        }
        self.has(host, viewer, "essentials.vanish.see")
    }
    pub fn is_vanished(&mut self, player: Uuid) -> bool {
        self.users.get(player).is_some_and(|u| u.bool("vanished"))
    }

    // ----- Dispatch ---------------------------------------------------------------------------------------------------
    /// Whether a command line is one Essentials runs (the glue claims it before vanilla's dispatcher).
    pub fn claims(&self, line: &str) -> bool {
        let label = line.trim_start_matches('/').split_whitespace().next().unwrap_or("");
        let disabled = self.config_list("disabled-commands").iter().any(|d| d.eq_ignore_ascii_case(label));
        !disabled && self.command_for(label).is_some()
    }

    /// Essentials.onCommandEssentials: permission, jail, run, then usage or error messages.
    pub fn dispatch(&mut self, host: &mut dyn Host, sender: Sender, line: &str) -> bool {
        let line = line.trim_start_matches('/').trim();
        let mut parts = line.split_whitespace();
        let Some(label) = parts.next() else { return false };
        let args: Vec<String> = parts.map(str::to_string).collect();
        let label_lower = label.to_lowercase();
        let Some(name) = self.command_for(&label_lower) else { return false };
        if label_lower == "essversion" {
            self.send_mini(host, sender, &format!("This server is running Essentials {}", env_version()));
            return true;
        }
        if self.config_list("disabled-commands").iter().any(|d| d.eq_ignore_ascii_case(&label_lower)) {
            self.send_tl(host, sender, "commandDisabled", &[Arg::Plain(label_lower)]);
            return true;
        }
        for hook in self.listeners.command.clone() {
            if hook(self, host, sender, line) == Allow::No {
                return true;
            }
        }
        if let Sender::Player(p) = sender {
            if name != "mail" && self.has(host, sender, "essentials.mail") {
                crate::game::ess_areas::notify_mail(self, host, p);
            }
            if !self.has(host, sender, &format!("essentials.{name}")) {
                let player = host.name(p).unwrap_or_default();
                self.send_tl(host, Sender::Console, "deniedAccessCommand", &[Arg::Plain(player)]);
                self.send_tl(host, sender, "noAccessCommand", &[]);
                return true;
            }
        }
        let Some(command) = self.commands.get(name).cloned() else { return false };
        let result = {
            let mut ctx = Ctx { ess: self, host, sender, label: label_lower.clone(), name };
            (command.run)(&mut ctx, &args)
        };
        match result {
            Ok(()) | Err(CmdError::Quiet) => {}
            Err(CmdError::Usage(extra)) => {
                let meta = ess_meta::command(name);
                if self.config_bool("verbose-command-usages", true) && !command.usages.is_empty() {
                    self.send_tl(host, sender, "commandHelpLine1", &[Arg::Plain(label_lower.clone())]);
                    let described = self.message(&format!("{name}CommandDescription"));
                    let description = if described.ends_with("CommandDescription") { meta.map_or("", |m| m.description).to_string() } else { described };
                    self.send_tl(host, sender, "commandHelpLine2", &[Arg::Parsed(description)]);
                    self.send_tl(host, sender, "commandHelpLine3", &[]);
                    for (usage, key) in command.usages {
                        let desc = self.tl(key, &[]);
                        self.send_tl(host, sender, "commandHelpLineUsage", &[Arg::Parsed(usage.replace("<command>", &label_lower)), Arg::Parsed(desc)]);
                    }
                } else if let Some(meta) = meta {
                    self.send_mini(host, sender, &ess_text::escape_tags(meta.description));
                    self.send_mini(host, sender, &ess_text::escape_tags(&meta.usage.replace("<command>", &label_lower)));
                }
                if let Some(extra) = extra {
                    self.send_mini(host, sender, &extra);
                }
            }
            Err(CmdError::PlayerNotFound) => {
                let msg = self.tl("playerNotFound", &[]);
                self.send_tl(host, sender, "errorWithMessage", &[Arg::Parsed(msg)]);
            }
            Err(CmdError::Tl(key, args)) => {
                let msg = self.tl(key, &args);
                self.send_tl(host, sender, "errorWithMessage", &[Arg::Parsed(msg)]);
            }
            Err(CmdError::Msg(m)) => self.send_tl(host, sender, "errorWithMessage", &[Arg::Plain(m)]),
        }
        true
    }

    // ----- Events -----------------------------------------------------------------------------------------------------
    pub fn on_join(&mut self, host: &mut dyn Host, player: Uuid) {
        let first = !self.users.exists(player);
        let name = host.name(player).unwrap_or_default();
        let now = host.now_millis();
        {
            let u = self.users.user(player);
            u.set("last-account-name", json!(name));
            u.set("timestamps.login", json!(now));
        }
        if let Some(ip) = host.address(player) {
            self.users.user(player).set("ip-address", json!(ip));
        }
        self.users.names.insert(name.to_lowercase(), player);
        if let Some(at) = host.location(player) {
            self.last_loc.insert(player, at);
        }
        for f in self.listeners.join.clone() {
            f(self, host, player, first);
        }
    }

    pub fn on_quit(&mut self, host: &mut dyn Host, player: Uuid) {
        let now = host.now_millis();
        if let Some(at) = host.location(player) {
            self.users.user(player).set("logoutlocation", at.to_yaml());
        }
        self.users.user(player).set("timestamps.logout", json!(now));
        for f in self.listeners.quit.clone() {
            f(self, host, player);
        }
        self.last_loc.remove(&player);
        self.users.save();
    }

    pub fn on_tick(&mut self, host: &mut dyn Host) {
        self.scheduler.tick += 1;
        for task in self.scheduler.due() {
            task(self, host);
        }
        for player in host.online() {
            let Some(now) = host.location(player) else { continue };
            match self.last_loc.insert(player, now.clone()) {
                Some(before) if before != now => {
                    for f in self.listeners.moved.clone() {
                        f(self, host, player, &before, &now);
                    }
                }
                _ => {}
            }
        }
        for f in self.listeners.tick.clone() {
            f(self, host);
        }
        // Userdata is written every 30 s and on quit (EssentialsX saves on change, asynchronously).
        if self.scheduler.tick % 600 == 0 {
            self.users.save();
        }
    }

    pub fn on_chat(&mut self, host: &mut dyn Host, player: Uuid, message: &str) -> Chat {
        for f in self.listeners.chat.clone() {
            let decision = f(self, host, player, message);
            if decision != Chat::Pass {
                return decision;
            }
        }
        Chat::Pass
    }

    pub fn on_death(&mut self, host: &mut dyn Host, player: Uuid, at: &Loc) {
        for f in self.listeners.death.clone() {
            f(self, host, player, at);
        }
    }

    pub fn on_respawn(&mut self, host: &mut dyn Host, player: Uuid) -> Option<Loc> {
        for f in self.listeners.respawn.clone() {
            if let Some(at) = f(self, host, player) {
                return Some(at);
            }
        }
        None
    }

    pub fn on_damage(&mut self, host: &mut dyn Host, player: Uuid, cause: &str, amount: f64) -> Allow {
        all_allow(self.listeners.damage.clone().into_iter().map(|f| f(self, host, player, cause, amount)))
    }
    pub fn on_block_break(&mut self, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], state: &str) -> Allow {
        all_allow(self.listeners.block_break.clone().into_iter().map(|f| f(self, host, player, world, pos, state)))
    }
    pub fn on_block_place(&mut self, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], state: &str) -> Allow {
        all_allow(self.listeners.block_place.clone().into_iter().map(|f| f(self, host, player, world, pos, state)))
    }
    pub fn on_sign_change(&mut self, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], lines: &mut [String; 4]) -> Allow {
        all_allow(self.listeners.sign_change.clone().into_iter().map(|f| f(self, host, player, world, pos, lines)))
    }
    pub fn on_interact(&mut self, host: &mut dyn Host, player: Uuid, block: Option<(&str, [i32; 3], &str)>, held: Option<&Item>) -> Allow {
        all_allow(self.listeners.interact.clone().into_iter().map(|f| f(self, host, player, block, held)))
    }
    pub fn on_container_click(&mut self, host: &mut dyn Host, player: Uuid, tag: &str, slot: usize) -> Allow {
        all_allow(self.listeners.container_click.clone().into_iter().map(|f| f(self, host, player, tag, slot)))
    }
    pub fn on_container_close(&mut self, host: &mut dyn Host, player: Uuid, tag: &str, items: &[Option<Item>]) {
        for f in self.listeners.container_close.clone() {
            f(self, host, player, tag, items);
        }
    }
    pub fn on_world(&mut self, host: &mut dyn Host, kind: &str, world: &str, pos: [i32; 3]) -> Allow {
        all_allow(self.listeners.world.clone().into_iter().map(|f| f(self, host, kind, world, pos)))
    }
}

/// Every listener runs (they may record things); any No wins.
fn all_allow(results: impl Iterator<Item = Allow>) -> Allow {
    let mut out = Allow::Yes;
    for r in results {
        if r == Allow::No {
            out = Allow::No;
        }
    }
    out
}

fn env_version() -> &'static str {
    "2.21.0-horizon"
}

fn cmd_ping(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        ctx.tl("pong", &[]);
    } else {
        let text = final_arg(args, 0);
        ctx.send_mini(&ess_text::escape_tags(&text));
    }
    Ok(())
}

/// Deep-merges `over` into `base` (config.yml over the defaults).
pub fn merge(base: &mut Value, over: Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(&k) {
                    Some(existing) if existing.is_object() && v.is_object() => merge(existing, v),
                    _ => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (b, o) => *b = o,
    }
}

/// java.util.Properties for message overrides in the data folder.
pub fn properties(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.is_empty() || t.starts_with('#') || t.starts_with('!') {
            continue;
        }
        let Some(i) = t.find(['=', ':']) else { continue };
        let key = t[..i].trim().replace("\\", "");
        let raw = t[i + 1..].trim_start();
        let mut value = String::new();
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                value.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => value.push('\n'),
                Some('t') => value.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        value.push(ch);
                    }
                }
                Some(o) => value.push(o),
                None => {}
            }
        }
        out.insert(key, value);
    }
    out
}

/// permissions.yml written on first start: the `default` group holds EssentialsX's "player-commands" (what a
/// regular player may use), `admin` everything. Ops get every op-default node anyway.
pub const DEFAULT_PERMISSIONS: &str = "\
# horizon-essentials permissions (EssentialsX expects a permissions plugin; this file is that plugin).
# Nodes are EssentialsX's (https://essentialsx.net/wiki/Command-Reference.html). `-node` denies; `a.*` grants a
# whole branch. Players without a group are in the group marked `default: true`. Operators also get every node
# whose plugin.yml default is `op`. Reload with /essentials reload.
groups:
  default:
    default: true
    prefix: ''
    suffix: ''
    permissions:
    - essentials.afk
    - essentials.afk.auto
    - essentials.back
    - essentials.back.ondeath
    - essentials.balance
    - essentials.balance.others
    - essentials.balancetop
    - essentials.build
    - essentials.chat.color
    - essentials.chat.format
    - essentials.chat.shout
    - essentials.chat.question
    - essentials.clearinventory
    - essentials.compass
    - essentials.depth
    - essentials.delhome
    - essentials.getpos
    - essentials.geoip.show
    - essentials.help
    - essentials.helpop
    - essentials.home
    - essentials.home.bed
    - essentials.homes
    - essentials.ignore
    - essentials.info
    - essentials.itemdb
    - essentials.kit
    - essentials.kits.tools
    - essentials.list
    - essentials.mail
    - essentials.mail.send
    - essentials.me
    - essentials.motd
    - essentials.msg
    - essentials.msg.color
    - essentials.nick
    - essentials.near
    - essentials.pay
    - essentials.ping
    - essentials.protect
    - essentials.r
    - essentials.rules
    - essentials.realname
    - essentials.renamehome
    - essentials.seen
    - essentials.sell
    - essentials.sethome
    - essentials.setxmpp
    - essentials.signs.create.protection
    - essentials.signs.create.trade
    - essentials.signs.break.protection
    - essentials.signs.break.trade
    - essentials.signs.use.*
    - essentials.spawn
    - essentials.suicide
    - essentials.time
    - essentials.tpa
    - essentials.tpaccept
    - essentials.tpahere
    - essentials.tpdeny
    - essentials.warp
    - essentials.warp.list
    - essentials.world
    - essentials.worth
    - essentials.xmpp
  admin:
    inherits:
    - default
    prefix: '&4[Admin] '
    suffix: ''
    permissions:
    - essentials.*
users: {}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_answers_pong() {
        assert_eq!(ping(""), "Pong!");
        assert_eq!(ping("  hello there "), "hello there");
    }

    #[test]
    fn uuids_round_trip() {
        let u: Uuid = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef;
        assert_eq!(uuid_string(u), "01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(parse_uuid(&uuid_string(u)), Some(u));
    }

    #[test]
    fn default_files_parse() {
        for (name, text) in crate::game::ess_defaults::FILES {
            if name.ends_with(".yml") {
                ess_yaml::parse(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            }
        }
        let config = ess_yaml::parse(crate::game::ess_defaults::file("config.yml").unwrap()).unwrap();
        assert_eq!(ess_yaml::get(&config, "message-colors.primary"), Some(&json!("#ffaa00")));
        assert!(ess_yaml::parse(DEFAULT_PERMISSIONS).is_ok());
    }

    #[test]
    fn permissions_resolve_groups_wildcards_and_negation() {
        let yaml = ess_yaml::parse(
            "groups:\n  default:\n    default: true\n    permissions: [essentials.home, -essentials.home.others]\n  vip:\n    inherits: [default]\n    permissions: ['essentials.sethome.multiple.*', essentials.fly]\nusers:\n  bob:\n    groups: [vip]\n    permissions: [-essentials.fly]\n",
        )
        .unwrap();
        let perms = Perms::load(&yaml);
        let (alice, bob) = (1u128, 2u128);
        assert_eq!(perms.configured(alice, "Alice", "essentials.home"), Some(true));
        assert_eq!(perms.configured(alice, "Alice", "essentials.home.others"), Some(false));
        assert_eq!(perms.configured(alice, "Alice", "essentials.fly"), None);
        assert_eq!(perms.configured(bob, "Bob", "essentials.sethome.multiple.vip"), Some(true));
        assert_eq!(perms.configured(bob, "Bob", "essentials.fly"), Some(false));
        assert_eq!(perms.groups_of(bob, "Bob"), vec!["vip".to_string()]);
    }

    #[test]
    fn plugin_yml_defaults() {
        assert!(default_grant("essentials.home", true));
        assert!(!default_grant("essentials.home", false));
        // default: false nodes are not granted to ops.
        assert!(!default_grant("essentials.keepinv", true));
    }

    #[test]
    fn scheduler_runs_and_cancels() {
        let mut s = Scheduler::default();
        let a = s.later(2, Box::new(|_, _| {}));
        let b = s.later(1, Box::new(|_, _| {}));
        s.cancel(a);
        s.tick = 1;
        assert_eq!(s.due().len(), 1);
        assert!(!s.pending(b));
        s.tick = 5;
        assert_eq!(s.due().len(), 0);
    }
}
