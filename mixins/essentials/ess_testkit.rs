// horizon-essentials test kit (tests only): FakeHost, an in-memory server implementing `Host`, and TestServer,
// Essentials over a FakeHost in a scratch data folder. Every area's tests use it:
//
//   let mut t = TestServer::new();
//   let alice = t.join("Alice");
//   t.op("Alice");
//   assert_eq!(t.cmd(alice, "/sethome base"), ["Home base set."]);
//   t.tick(60);
//
// Lines are compared as plain text (styles dropped); `t.host.spans` keeps the styled lines.

#[cfg(test)]
pub use self::kit::*;

#[cfg(test)]
mod kit {
    use crate::game::ess_core::*;
    use crate::game::ess_text::plain;
    use std::collections::{BTreeMap, HashMap};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// One fake player and everything Host can read or change about them.
    #[derive(Clone, Debug)]
    pub struct FakePlayer {
        pub name: String,
        pub online: bool,
        pub loc: Loc,
        pub op: u8,
        pub mode: GameMode,
        pub health: f64,
        pub max_health: f64,
        pub food: i32,
        pub saturation: f32,
        pub fire: i32,
        pub freeze: i32,
        pub exp: i64,
        pub flying: bool,
        pub allow_flight: bool,
        pub fly_speed: f32,
        pub walk_speed: f32,
        pub invulnerable: bool,
        pub hidden: bool,
        pub display_name: Option<String>,
        pub tab_name: Option<String>,
        pub ptime: Option<(i64, bool)>,
        pub pweather: Option<bool>,
        pub inventory: Vec<Option<Item>>,
        pub held: usize,
        pub ender: Vec<Option<Item>>,
        pub open: Option<Container>,
        pub station: Option<Station>,
        pub address: String,
        pub bed: Option<Loc>,
        pub stats: HashMap<String, i64>,
        /// Where the player looks (target_block answers this).
        pub target: Option<[i32; 3]>,
    }

    /// A deterministic UUID from a name (stands in for vanilla's offline UUID).
    pub fn fake_uuid(name: &str) -> Uuid {
        let mut h: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
        for b in name.to_lowercase().bytes() {
            h ^= b as u128;
            h = h.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
        }
        h
    }

    pub struct FakeHost {
        pub now: i64,
        pub players: BTreeMap<Uuid, FakePlayer>,
        /// Plain-text lines each receiver got, in order (drained by `said`).
        pub inbox: HashMap<Sender, Vec<String>>,
        /// The same lines with styles.
        pub spans: HashMap<Sender, Vec<Vec<Span>>>,
        pub action_bars: HashMap<Uuid, Vec<String>>,
        /// Text shown as a player's chat (chat_as), with who "sent" it.
        pub chats: Vec<(Uuid, String)>,
        pub kicked: Vec<(Uuid, String)>,
        /// Blocks changed from the flat world (stone up to y 63, bedrock at -64, air above).
        pub blocks: HashMap<(String, [i32; 3]), String>,
        pub spawns: HashMap<String, Loc>,
        pub signs: HashMap<(String, [i32; 3]), [String; 4]>,
        pub time: HashMap<String, i64>,
        pub weather: HashMap<String, (bool, bool, Option<i32>)>,
        pub entities: Vec<(String, Loc, Option<String>)>,
        pub lightning: Vec<(Loc, bool)>,
        pub dropped: Vec<(Loc, Item)>,
        pub console_commands: Vec<String>,
        pub player_commands: Vec<(Uuid, String, bool)>,
        pub bans: Vec<Ban>,
        pub ip_bans: Vec<Ban>,
        pub motd: Option<String>,
        pub dir: PathBuf,
        /// item id -> (max stack, max damage)
        pub items: BTreeMap<String, (i32, i32)>,
        pub enchantments: Vec<(String, i32)>,
        pub entity_types: Vec<String>,
        pub stats: ServerStats,
        pub max_players: usize,
        /// Per-area fake state (areas add fields only in their region; initialize them in `new`'s region).
        // ---- area tp ----
        /// Players on the death screen (Host::is_dead).
        pub dead: std::collections::HashSet<Uuid>,
        // ---- end tp ----
        // ---- area social ----
        // ---- end social ----
        // ---- area admin ----
        /// Players on the whitelist.
        pub whitelist: Vec<Uuid>,
        /// First join times (Unix millis).
        pub first_played: HashMap<Uuid, i64>,
        /// Server log lines (Host::log), plain.
        pub logs: Vec<String>,
        // ---- end admin ----
        // ---- area items ----
        // ---- end items ----
        // ---- area economy ----
        /// Players holding sneak (bulk sign trades).
        pub economy_sneaking: std::collections::HashSet<Uuid>,
        // ---- end economy ----
        // ---- area world ----
        // ---- end world ----
        pub _regions: (),
    }

    impl FakeHost {
        pub fn new(dir: PathBuf) -> Self {
            let mut items = BTreeMap::new();
            for (id, stack, damage) in [
                ("stone", 64, 0), ("cobblestone", 64, 0), ("dirt", 64, 0), ("grass_block", 64, 0), ("oak_log", 64, 0),
                ("oak_planks", 64, 0), ("stick", 64, 0), ("torch", 64, 0), ("diamond", 64, 0), ("diamond_block", 64, 0),
                ("iron_ingot", 64, 0), ("iron_block", 64, 0), ("iron_nugget", 64, 0), ("gold_ingot", 64, 0),
                ("gold_block", 64, 0), ("gold_nugget", 64, 0), ("emerald", 64, 0), ("emerald_block", 64, 0),
                ("coal", 64, 0), ("coal_block", 64, 0), ("redstone", 64, 0), ("redstone_block", 64, 0),
                ("lapis_lazuli", 64, 0), ("lapis_block", 64, 0), ("netherite_ingot", 64, 0), ("netherite_block", 64, 0),
                ("copper_ingot", 64, 0), ("copper_block", 64, 0), ("wheat", 64, 0), ("hay_block", 64, 0),
                ("bread", 64, 0), ("apple", 64, 0), ("golden_apple", 64, 0), ("cooked_beef", 64, 0), ("arrow", 64, 0),
                ("tnt", 64, 0), ("chest", 64, 0), ("ender_pearl", 16, 0), ("snowball", 16, 0), ("egg", 16, 0),
                ("oak_sign", 16, 0), ("written_book", 16, 0), ("writable_book", 1, 0), ("player_head", 64, 0),
                ("firework_rocket", 64, 0), ("potion", 1, 0), ("splash_potion", 1, 0), ("bucket", 16, 0),
                ("water_bucket", 1, 0), ("compass", 64, 0), ("clock", 64, 0), ("bow", 1, 384), ("shield", 1, 336),
                ("elytra", 1, 432), ("diamond_sword", 1, 1561), ("diamond_pickaxe", 1, 1561), ("diamond_axe", 1, 1561),
                ("diamond_shovel", 1, 1561), ("diamond_helmet", 1, 363), ("diamond_chestplate", 1, 528),
                ("diamond_leggings", 1, 495), ("diamond_boots", 1, 429), ("iron_sword", 1, 250), ("iron_pickaxe", 1, 250),
                ("stone_sword", 1, 131), ("stone_pickaxe", 1, 131), ("stone_axe", 1, 131), ("stone_shovel", 1, 131),
                ("leather_helmet", 1, 55), ("leather_chestplate", 1, 80), ("leather_leggings", 1, 75),
                ("leather_boots", 1, 65), ("fishing_rod", 1, 64), ("flint_and_steel", 1, 64), ("shears", 1, 238),
                ("carved_pumpkin", 64, 0), ("spawner", 64, 0), ("bedrock", 64, 0), ("barrier", 64, 0), ("air", 64, 0),
            ] {
                items.insert(format!("minecraft:{id}"), (stack, damage));
            }
            let enchantments = [
                ("sharpness", 5), ("smite", 5), ("bane_of_arthropods", 5), ("knockback", 2), ("fire_aspect", 2),
                ("looting", 3), ("sweeping_edge", 3), ("efficiency", 5), ("silk_touch", 1), ("unbreaking", 3),
                ("fortune", 3), ("power", 5), ("punch", 2), ("flame", 1), ("infinity", 1), ("protection", 4),
                ("fire_protection", 4), ("feather_falling", 4), ("blast_protection", 4), ("projectile_protection", 4),
                ("respiration", 3), ("aqua_affinity", 1), ("thorns", 3), ("depth_strider", 3), ("frost_walker", 2),
                ("mending", 1), ("binding_curse", 1), ("vanishing_curse", 1), ("luck_of_the_sea", 3), ("lure", 3),
            ]
            .iter()
            .map(|(e, l)| (format!("minecraft:{e}"), *l))
            .collect();
            let entity_types = [
                "zombie", "skeleton", "creeper", "spider", "enderman", "slime", "cow", "pig", "sheep", "chicken",
                "villager", "cat", "wolf", "horse", "bee", "iron_golem", "tnt", "lightning_bolt", "item",
                "experience_orb", "arrow", "fireball", "small_fireball", "dragon_fireball", "wither_skull",
                "snowball", "egg", "trident", "potion", "experience_bottle", "ender_dragon", "wither", "armor_stand",
            ]
            .iter()
            .map(|e| format!("minecraft:{e}"))
            .collect();
            let mut spawns = HashMap::new();
            spawns.insert("minecraft:overworld".to_string(), Loc::new("minecraft:overworld", 0.5, 64.0, 0.5));
            FakeHost {
                now: 1_700_000_000_000,
                players: BTreeMap::new(),
                inbox: HashMap::new(),
                spans: HashMap::new(),
                action_bars: HashMap::new(),
                chats: Vec::new(),
                kicked: Vec::new(),
                blocks: HashMap::new(),
                spawns,
                signs: HashMap::new(),
                time: HashMap::new(),
                weather: HashMap::new(),
                entities: Vec::new(),
                lightning: Vec::new(),
                dropped: Vec::new(),
                console_commands: Vec::new(),
                player_commands: Vec::new(),
                bans: Vec::new(),
                ip_bans: Vec::new(),
                motd: None,
                dir,
                items,
                enchantments,
                entity_types,
                stats: ServerStats { tps: 20.0, mspt: 12.5, uptime_ms: 3_600_000, memory_used_mb: 512, memory_max_mb: 2048, worlds: vec![("minecraft:overworld".into(), 441, 12, 3)] },
                max_players: 20,
                // ---- area tp ----
                dead: std::collections::HashSet::new(),
                // ---- end tp ----
                // ---- area social ----
                // ---- end social ----
                // ---- area admin ----
                whitelist: Vec::new(),
                first_played: HashMap::new(),
                logs: Vec::new(),
                // ---- end admin ----
                // ---- area items ----
                // ---- end items ----
                // ---- area economy ----
                economy_sneaking: Default::default(),
                // ---- end economy ----
                // ---- area world ----
                // ---- end world ----
                _regions: (),
            }
        }

        pub fn add_player(&mut self, name: &str) -> Uuid {
            let uuid = fake_uuid(name);
            let spawn = self.spawns.get("minecraft:overworld").cloned().unwrap_or_else(|| Loc::new("minecraft:overworld", 0.5, 64.0, 0.5));
            let player = self.players.entry(uuid).or_insert_with(|| FakePlayer {
                name: name.to_string(),
                online: true,
                loc: spawn,
                op: 0,
                mode: GameMode::Survival,
                health: 20.0,
                max_health: 20.0,
                food: 20,
                saturation: 5.0,
                fire: 0,
                freeze: 0,
                exp: 0,
                flying: false,
                allow_flight: false,
                fly_speed: 0.1,
                walk_speed: 0.2,
                invulnerable: false,
                hidden: false,
                display_name: None,
                tab_name: None,
                ptime: None,
                pweather: None,
                inventory: vec![None; 41],
                held: 0,
                ender: vec![None; 27],
                open: None,
                station: None,
                address: format!("127.0.0.{}", (uuid % 200) + 2),
                bed: None,
                stats: HashMap::new(),
                target: None,
            });
            player.online = true;
            uuid
        }

        pub fn p(&mut self, player: Uuid) -> &mut FakePlayer {
            self.players.get_mut(&player).expect("a fake player")
        }

        /// Drains the plain lines a receiver got since the last call.
        pub fn said(&mut self, to: Sender) -> Vec<String> {
            self.inbox.remove(&to).unwrap_or_default()
        }

        fn ground(&self, world: &str, pos: [i32; 3]) -> String {
            if let Some(b) = self.blocks.get(&(world.to_string(), pos)) {
                return b.clone();
            }
            match pos[1] {
                y if y <= -64 => "minecraft:bedrock".into(),
                y if y <= 63 => "minecraft:stone".into(),
                _ => "minecraft:air".into(),
            }
        }
    }

    impl Host for FakeHost {
        fn now_millis(&mut self) -> i64 {
            self.now
        }
        fn online(&mut self) -> Vec<Uuid> {
            self.players.iter().filter(|(_, p)| p.online).map(|(u, _)| *u).collect()
        }
        fn name(&mut self, player: Uuid) -> Option<String> {
            self.players.get(&player).map(|p| p.name.clone())
        }
        fn lookup(&mut self, name: &str) -> Option<(Uuid, String)> {
            self.players.iter().find(|(_, p)| p.name.eq_ignore_ascii_case(name)).map(|(u, p)| (*u, p.name.clone()))
        }
        fn op_level(&mut self, player: Uuid) -> u8 {
            self.players.get(&player).map_or(0, |p| p.op)
        }
        fn address(&mut self, player: Uuid) -> Option<String> {
            self.players.get(&player).map(|p| p.address.clone())
        }
        fn kick(&mut self, player: Uuid, reason: &[Span]) {
            self.kicked.push((player, plain(reason)));
            if let Some(p) = self.players.get_mut(&player) {
                p.online = false;
            }
        }
        fn send(&mut self, to: Sender, text: &[Span]) {
            self.inbox.entry(to).or_default().push(plain(text));
            self.spans.entry(to).or_default().push(text.to_vec());
        }
        fn action_bar(&mut self, player: Uuid, text: &[Span]) {
            self.action_bars.entry(player).or_default().push(plain(text));
        }
        fn chat_as(&mut self, player: Uuid, text: &[Span]) {
            self.chats.push((player, plain(text)));
            self.broadcast(text);
        }
        fn location(&mut self, player: Uuid) -> Option<Loc> {
            self.players.get(&player).filter(|p| p.online).map(|p| p.loc.clone())
        }
        fn teleport(&mut self, player: Uuid, to: &Loc) -> bool {
            match self.players.get_mut(&player) {
                Some(p) if p.online => {
                    p.loc = to.clone();
                    true
                }
                _ => false,
            }
        }
        fn world_spawn(&mut self, world: &str) -> Option<Loc> {
            self.spawns.get(world).cloned().or_else(|| Some(Loc::new(world, 0.5, 64.0, 0.5)))
        }
        fn set_world_spawn(&mut self, at: &Loc) {
            self.spawns.insert(at.world.clone(), at.clone());
        }
        fn bed_spawn(&mut self, player: Uuid) -> Option<Loc> {
            self.players.get(&player).and_then(|p| p.bed.clone())
        }
        fn block(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
            Some(self.ground(world, pos))
        }
        fn set_block(&mut self, world: &str, pos: [i32; 3], state: &str) -> bool {
            self.blocks.insert((world.to_string(), pos), state.to_string());
            true
        }
        fn highest_y(&mut self, world: &str, x: i32, z: i32) -> Option<i32> {
            let (min, max) = self.height_range(world);
            (min..max).rev().find(|y| !self.ground(world, [x, *y, z]).ends_with(":air"))
        }
        fn target_block(&mut self, player: Uuid, _max: f64) -> Option<[i32; 3]> {
            self.players.get(&player).and_then(|p| p.target)
        }
        fn biome(&mut self, _world: &str, _pos: [i32; 3]) -> Option<String> {
            Some("minecraft:plains".into())
        }
        fn sign_lines(&mut self, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
            self.signs.get(&(world.to_string(), pos)).cloned()
        }
        fn set_sign_lines(&mut self, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
            self.signs.insert((world.to_string(), pos), lines.clone());
            true
        }
        fn game_mode(&mut self, player: Uuid) -> GameMode {
            self.players.get(&player).map_or(GameMode::Survival, |p| p.mode)
        }
        fn set_game_mode(&mut self, player: Uuid, mode: GameMode) {
            if let Some(p) = self.players.get_mut(&player) {
                p.mode = mode;
                p.allow_flight = matches!(mode, GameMode::Creative | GameMode::Spectator);
            }
        }
        fn health(&mut self, player: Uuid) -> (f64, f64) {
            self.players.get(&player).map_or((20.0, 20.0), |p| (p.health, p.max_health))
        }
        fn set_health(&mut self, player: Uuid, health: f64) {
            if let Some(p) = self.players.get_mut(&player) {
                p.health = health.clamp(0.0, p.max_health);
            }
        }
        fn food(&mut self, player: Uuid) -> (i32, f32) {
            self.players.get(&player).map_or((20, 5.0), |p| (p.food, p.saturation))
        }
        fn set_food(&mut self, player: Uuid, food: i32, saturation: f32) {
            if let Some(p) = self.players.get_mut(&player) {
                p.food = food;
                p.saturation = saturation;
            }
        }
        fn set_fire_ticks(&mut self, player: Uuid, ticks: i32) {
            if let Some(p) = self.players.get_mut(&player) {
                p.fire = ticks;
            }
        }
        fn fire_ticks(&mut self, player: Uuid) -> i32 {
            self.players.get(&player).map_or(0, |p| p.fire)
        }
        fn set_freeze_ticks(&mut self, player: Uuid, ticks: i32) {
            if let Some(p) = self.players.get_mut(&player) {
                p.freeze = ticks;
            }
        }
        fn exp(&mut self, player: Uuid) -> i64 {
            self.players.get(&player).map_or(0, |p| p.exp)
        }
        fn set_exp(&mut self, player: Uuid, total: i64) {
            if let Some(p) = self.players.get_mut(&player) {
                p.exp = total.max(0);
            }
        }
        fn flying(&mut self, player: Uuid) -> bool {
            self.players.get(&player).is_some_and(|p| p.flying)
        }
        fn set_allow_flight(&mut self, player: Uuid, allow: bool) {
            if let Some(p) = self.players.get_mut(&player) {
                p.allow_flight = allow;
                if !allow {
                    p.flying = false;
                }
            }
        }
        fn allow_flight(&mut self, player: Uuid) -> bool {
            self.players.get(&player).is_some_and(|p| p.allow_flight)
        }
        fn set_flying(&mut self, player: Uuid, flying: bool) {
            if let Some(p) = self.players.get_mut(&player) {
                p.flying = flying && p.allow_flight;
            }
        }
        fn set_fly_speed(&mut self, player: Uuid, speed: f32) {
            if let Some(p) = self.players.get_mut(&player) {
                p.fly_speed = speed;
            }
        }
        fn set_walk_speed(&mut self, player: Uuid, speed: f32) {
            if let Some(p) = self.players.get_mut(&player) {
                p.walk_speed = speed;
            }
        }
        fn set_invulnerable(&mut self, player: Uuid, invulnerable: bool) {
            if let Some(p) = self.players.get_mut(&player) {
                p.invulnerable = invulnerable;
            }
        }
        fn set_hidden(&mut self, player: Uuid, hidden: bool, _seers: &[Uuid]) {
            if let Some(p) = self.players.get_mut(&player) {
                p.hidden = hidden;
            }
        }
        fn set_display_name(&mut self, player: Uuid, name: Option<&[Span]>) {
            if let Some(p) = self.players.get_mut(&player) {
                p.display_name = name.map(plain);
            }
        }
        fn set_tab_name(&mut self, player: Uuid, name: Option<&[Span]>) {
            if let Some(p) = self.players.get_mut(&player) {
                p.tab_name = name.map(plain);
            }
        }
        fn set_player_time(&mut self, player: Uuid, time: Option<(i64, bool)>) {
            if let Some(p) = self.players.get_mut(&player) {
                p.ptime = time;
            }
        }
        fn set_player_weather(&mut self, player: Uuid, downfall: Option<bool>) {
            if let Some(p) = self.players.get_mut(&player) {
                p.pweather = downfall;
            }
        }
        fn statistic(&mut self, player: Uuid, stat: &str) -> i64 {
            self.players.get(&player).and_then(|p| p.stats.get(stat).copied()).unwrap_or(0)
        }
        fn inventory(&mut self, player: Uuid) -> Vec<Option<Item>> {
            self.players.get(&player).map_or_else(|| vec![None; 41], |p| p.inventory.clone())
        }
        fn set_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
            if let Some(p) = self.players.get_mut(&player) {
                if slot < p.inventory.len() {
                    p.inventory[slot] = item.filter(|i| i.count > 0);
                }
            }
        }
        fn held_slot(&mut self, player: Uuid) -> usize {
            self.players.get(&player).map_or(0, |p| p.held)
        }
        fn ender_chest(&mut self, player: Uuid) -> Vec<Option<Item>> {
            self.players.get(&player).map_or_else(|| vec![None; 27], |p| p.ender.clone())
        }
        fn set_ender_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
            if let Some(p) = self.players.get_mut(&player) {
                if slot < p.ender.len() {
                    p.ender[slot] = item.filter(|i| i.count > 0);
                }
            }
        }
        fn open_container(&mut self, player: Uuid, container: &Container) -> bool {
            match self.players.get_mut(&player) {
                Some(p) if p.online => {
                    p.open = Some(container.clone());
                    true
                }
                _ => false,
            }
        }
        fn open_station(&mut self, player: Uuid, station: Station) -> bool {
            match self.players.get_mut(&player) {
                Some(p) if p.online => {
                    p.station = Some(station);
                    true
                }
                _ => false,
            }
        }
        fn close_container(&mut self, player: Uuid) {
            if let Some(p) = self.players.get_mut(&player) {
                p.open = None;
                p.station = None;
            }
        }
        fn drop_item(&mut self, at: &Loc, item: &Item) {
            self.dropped.push((at.clone(), item.clone()));
        }
        fn item_ids(&mut self) -> Vec<String> {
            self.items.keys().cloned().collect()
        }
        fn max_stack(&mut self, item: &str) -> i32 {
            self.items.get(item).map_or(64, |i| i.0)
        }
        fn max_damage(&mut self, item: &str) -> i32 {
            self.items.get(item).map_or(0, |i| i.1)
        }
        fn enchantments(&mut self) -> Vec<(String, i32)> {
            self.enchantments.clone()
        }
        fn entity_types(&mut self) -> Vec<String> {
            self.entity_types.clone()
        }
        fn world_time(&mut self, world: &str) -> i64 {
            self.time.get(world).copied().unwrap_or(0)
        }
        fn set_world_time(&mut self, world: &str, time: i64) {
            self.time.insert(world.to_string(), time);
        }
        fn weather(&mut self, world: &str) -> (bool, bool) {
            self.weather.get(world).map_or((false, false), |w| (w.0, w.1))
        }
        fn set_weather(&mut self, world: &str, rain: bool, thunder: bool, duration: Option<i32>) {
            self.weather.insert(world.to_string(), (rain, thunder, duration));
        }
        fn spawn_entity(&mut self, kind: &str, at: &Loc, snbt: Option<&str>) -> bool {
            self.entities.push((kind.to_string(), at.clone(), snbt.map(str::to_string)));
            true
        }
        fn strike_lightning(&mut self, at: &Loc, effect_only: bool) {
            self.lightning.push((at.clone(), effect_only));
        }
        fn remove_entities(&mut self, world: &str, center: Option<&Loc>, radius: Option<f64>, kinds: &[String]) -> usize {
            let before = self.entities.len();
            self.entities.retain(|(kind, at, _)| {
                let in_world = at.world == world;
                let near = match (center, radius) {
                    (Some(c), Some(r)) => c.distance(at) <= r,
                    _ => true,
                };
                let kind_ok = kinds.is_empty() || kinds.iter().any(|k| k == kind);
                !(in_world && near && kind_ok)
            });
            before - self.entities.len()
        }
        fn console_command(&mut self, line: &str) -> (bool, Vec<String>) {
            self.console_commands.push(line.trim_start_matches('/').to_string());
            (true, Vec::new())
        }
        fn player_command(&mut self, player: Uuid, line: &str, op: bool) -> (bool, Vec<String>) {
            self.player_commands.push((player, line.trim_start_matches('/').to_string(), op));
            (true, Vec::new())
        }
        fn stats(&mut self) -> ServerStats {
            self.stats.clone()
        }
        fn max_players(&mut self) -> usize {
            self.max_players
        }
        fn set_motd(&mut self, motd: &[Span]) {
            self.motd = Some(plain(motd));
        }
        fn bans(&mut self, ip: bool) -> Vec<Ban> {
            if ip { self.ip_bans.clone() } else { self.bans.clone() }
        }
        fn ban(&mut self, ip: bool, ban: &Ban) {
            let list = if ip { &mut self.ip_bans } else { &mut self.bans };
            list.retain(|b| !b.target.eq_ignore_ascii_case(&ban.target));
            list.push(ban.clone());
        }
        fn unban(&mut self, ip: bool, target: &str) -> bool {
            let list = if ip { &mut self.ip_bans } else { &mut self.bans };
            let before = list.len();
            list.retain(|b| !b.target.eq_ignore_ascii_case(target));
            before != list.len()
        }
        fn server_dir(&mut self) -> PathBuf {
            self.dir.clone()
        }

        // ---- area tp ----
        fn is_dead(&mut self, player: Uuid) -> bool {
            self.dead.contains(&player)
        }
        // ---- end tp ----
        // ---- area social ----
        // ---- end social ----
        // ---- area admin ----
        fn whitelisted(&mut self, player: Uuid) -> bool {
            self.whitelist.contains(&player)
        }
        fn first_played(&mut self, player: Uuid) -> Option<i64> {
            self.first_played.get(&player).copied()
        }
        fn speeds(&mut self, player: Uuid) -> (f32, f32) {
            self.players.get(&player).map_or((0.1, 0.2), |p| (p.fly_speed, p.walk_speed))
        }
        fn log(&mut self, text: &[Span]) {
            self.logs.push(plain(text));
        }
        // ---- end admin ----
        // ---- area items ----
        // ---- end items ----
        // ---- area economy ----
        fn sneaking(&mut self, player: Uuid) -> bool {
            self.economy_sneaking.contains(&player)
        }
        fn item_is_block(&mut self, item: &str) -> bool {
            let id = item.trim_start_matches("minecraft:");
            id.ends_with("_block") || id.ends_with("_log") || id.ends_with("_planks") || id.ends_with("_sign")
                || matches!(id, "stone" | "cobblestone" | "dirt" | "torch" | "tnt" | "chest" | "carved_pumpkin" | "spawner" | "bedrock" | "barrier" | "player_head")
        }
        // ---- end economy ----
        // ---- area world ----
        // ---- end world ----
    }

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    /// Essentials over a FakeHost, in a scratch server folder removed on drop.
    pub struct TestServer {
        pub ess: Ess,
        pub host: FakeHost,
        pub dir: PathBuf,
    }

    impl TestServer {
        pub fn new() -> Self {
            Self::with_config("")
        }

        /// A server whose config.yml has these YAML lines on top of EssentialsX's defaults
        /// (e.g. "teleport-delay: 3\nteleport-cooldown: 0").
        pub fn with_config(config: &str) -> Self {
            let n = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("ess-test-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let data = dir.join("essentials");
            std::fs::create_dir_all(&data).expect("scratch folder");
            if !config.is_empty() {
                let mut base = crate::game::ess_yaml::parse(crate::game::ess_defaults::file("config.yml").unwrap_or("")).unwrap_or_default();
                let over = crate::game::ess_yaml::parse(config).expect("test config is YAML");
                merge(&mut base, over);
                std::fs::write(data.join("config.yml"), crate::game::ess_yaml::to_string(&base)).expect("config.yml");
            }
            let host = FakeHost::new(dir.clone());
            let ess = Ess::new(&data);
            TestServer { ess, host, dir }
        }

        /// A player joins (Essentials' join handling runs); returns their uuid.
        pub fn join(&mut self, name: &str) -> Uuid {
            let uuid = self.host.add_player(name);
            self.ess.on_join(&mut self.host, uuid);
            uuid
        }

        pub fn quit(&mut self, player: Uuid) {
            self.ess.on_quit(&mut self.host, player);
            self.host.p(player).online = false;
        }

        /// Makes a player a vanilla operator (level 4).
        pub fn op(&mut self, name: &str) {
            let uuid = fake_uuid(name);
            self.host.p(uuid).op = 4;
        }

        /// Writes permissions.yml (YAML) and reloads it.
        pub fn permissions(&mut self, yaml: &str) {
            std::fs::write(self.ess.dir.join("permissions.yml"), yaml).expect("permissions.yml");
            self.ess.reload();
        }

        /// A player runs a command line; returns the lines they received meanwhile (drained).
        pub fn cmd(&mut self, player: Uuid, line: &str) -> Vec<String> {
            self.host.said(Sender::Player(player));
            let handled = self.ess.dispatch(&mut self.host, Sender::Player(player), line);
            assert!(handled, "Essentials does not handle {line:?}");
            self.host.said(Sender::Player(player))
        }

        /// The console runs a command line; returns the console's lines.
        pub fn console(&mut self, line: &str) -> Vec<String> {
            self.host.said(Sender::Console);
            let handled = self.ess.dispatch(&mut self.host, Sender::Console, line);
            assert!(handled, "Essentials does not handle {line:?}");
            self.host.said(Sender::Console)
        }

        /// Lines a player got since last asked.
        pub fn said(&mut self, player: Uuid) -> Vec<String> {
            self.host.said(Sender::Player(player))
        }

        /// A player sends chat; returns Essentials' decision.
        pub fn chat(&mut self, player: Uuid, message: &str) -> Chat {
            self.ess.on_chat(&mut self.host, player, message)
        }

        /// Runs server ticks (the scheduler, movement checks, tick listeners); the clock advances 50 ms each.
        pub fn tick(&mut self, ticks: u32) {
            for _ in 0..ticks {
                self.host.now += 50;
                self.ess.on_tick(&mut self.host);
            }
        }

        /// Moves a player as their client would (movement listeners see it on the next tick).
        pub fn walk(&mut self, player: Uuid, to: Loc) {
            self.host.p(player).loc = to;
            self.tick(1);
        }

        pub fn loc(&mut self, player: Uuid) -> Loc {
            self.host.p(player).loc.clone()
        }

        /// The player's userdata file as Essentials wrote it (after a save).
        pub fn userdata(&mut self, player: Uuid) -> serde_json::Value {
            self.ess.users.save();
            let path = self.ess.dir.join("userdata").join(format!("{}.yml", uuid_string(player)));
            crate::game::ess_yaml::parse(&std::fs::read_to_string(path).unwrap_or_default()).unwrap_or_default()
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn ping_through_the_dispatcher() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        assert_eq!(t.cmd(alice, "/ping"), ["Pong!"]);
        assert_eq!(t.cmd(alice, "/echo hi <b>there"), ["hi <b>there"]);
        assert_eq!(t.console("eping"), ["Pong!"]);
    }

    #[test]
    fn permission_denied_without_the_node() {
        let mut t = TestServer::new();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [-essentials.ping]\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/ping"), ["You do not have access to that command."]);
        assert_eq!(t.host.said(Sender::Console), ["Bob was denied access to command."]);
    }

    #[test]
    fn join_writes_userdata() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let data = t.userdata(alice);
        assert_eq!(data["last-account-name"], "Alice");
        assert_eq!(data["timestamps"]["login"], 1_700_000_000_000i64);
    }

    #[test]
    fn scheduler_tasks_run_on_ticks() {
        let mut t = TestServer::with_config("newbies:\n  announce-format: ''\n");
        let alice = t.join("Alice");
        t.said(alice); // the join's own lines (motd, mail notice)
        t.ess.scheduler.later(3, Box::new(move |ess, host| ess.send_mini(host, Sender::Player(alice), "later")));
        t.tick(2);
        assert!(t.said(alice).is_empty());
        t.tick(1);
        assert_eq!(t.said(alice), ["later"]);
    }

    #[test]
    fn match_player_by_prefix_and_offline() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bobby");
        assert_eq!(t.ess.match_player(&mut t.host, Sender::Console, "bob", false), Some(bob));
        t.quit(alice);
        assert_eq!(t.ess.match_player(&mut t.host, Sender::Console, "Alice", false), None);
        assert_eq!(t.ess.match_player(&mut t.host, Sender::Console, "alice", true), Some(alice));
    }
}
