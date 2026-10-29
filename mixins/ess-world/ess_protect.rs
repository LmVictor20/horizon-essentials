// horizon-essentials area `world`, module `ess_protect` (pure logic, standalone-tested): EssentialsX Protect +
// AntiBuild. Ports EssentialsProtect{Entity,Block,Weather}Listener and EssentialsAntiBuildListener as decisions
// on the core's `damage`, `world`, `block_place`, `block_break` and `interact` events (config `protect.*`).
//
// World event kinds this module understands (the glue emits what Horizon lets it see):
//   `ignite:<spread|flint_and_steel|lava|lightning|fireball>` (`fire_spread` = ignite:spread), `burn`, `portal`,
//   `flow:<water|lava|air>` (`flow` = look at the block), `explode:<entity>` (`explode` alone: unknown source),
//   `spawn:<entity>`, `change_block:<entity>`, `door_break:<entity>`, `weather:<storm|thunder>`, `lightning`,
//   `transform:<kind>`, `villager_damage`, `bed_explode`, `item_damage:<cause>`, `piston:<block>`,
//   `dispense:<item>`, `target:<player uuid>`.
// Damage causes: see ess_world.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    ess.listeners.damage.push(on_damage);
    ess.listeners.world.push(on_world);
    ess.listeners.block_place.push(on_place);
    ess.listeners.block_break.push(on_break);
    ess.listeners.interact.push(on_interact);
}

fn prevent(ess: &Ess, key: &str) -> bool {
    ess.config_bool(&format!("protect.prevent.{key}"), false)
}

fn disable(ess: &Ess, key: &str) -> bool {
    ess.config_bool(&format!("protect.disable.{key}"), false)
}

/// shouldBeDamaged: essentials.protect.damage.<type> and not essentials.protect.damage.disable.
fn should_be_damaged(ess: &mut Ess, host: &mut dyn Host, player: Uuid, kind: &str) -> bool {
    let who = Sender::Player(player);
    ess.has(host, who, &format!("essentials.protect.damage.{kind}")) && !ess.has(host, who, "essentials.protect.damage.disable")
}

fn pvp_blocked(ess: &mut Ess, host: &mut dyn Host, target: Uuid, attacker: Uuid) -> bool {
    disable(ess, "pvp")
        && target != attacker
        && (!ess.has(host, Sender::Player(target), "essentials.protect.pvp") || !ess.has(host, Sender::Player(attacker), "essentials.protect.pvp"))
}

fn on_damage(ess: &mut Ess, host: &mut dyn Host, player: Uuid, cause: &str, _amount: f64) -> Allow {
    let (head, tail) = cause.split_once(':').unwrap_or((cause, ""));
    let guarded = |ess: &mut Ess, host: &mut dyn Host, on: bool, kind: &str| on && !should_be_damaged(ess, host, player, kind);
    let blocked = match (head, tail) {
        ("contact", _) => guarded(ess, host, disable(ess, "contactdmg"), "contact"),
        ("lava", _) => guarded(ess, host, disable(ess, "lavadmg"), "lava"),
        ("explosion", "tnt") => {
            let on = prevent(ess, "tnt-explosion") || prevent(ess, "tnt-playerdamage");
            guarded(ess, host, on, "tnt")
        }
        ("explosion", "creeper") => guarded(ess, host, prevent(ess, "creeper-playerdamage"), "creeper"),
        ("explosion", "fireball" | "small_fireball") => guarded(ess, host, prevent(ess, "fireball-playerdamage"), "fireball"),
        ("explosion", "wither_skull") => guarded(ess, host, prevent(ess, "witherskull-playerdamage"), "witherskull"),
        ("explosion", "tnt_minecart") => guarded(ess, host, prevent(ess, "tnt-minecart-playerdamage"), "tnt-minecart"),
        ("player", who) => match parse_uuid(who) {
            Some(attacker) => pvp_blocked(ess, host, player, attacker),
            None => false,
        },
        ("projectile", shooter) => {
            let by_type = guarded(ess, host, disable(ess, "projectiles"), "projectiles");
            let pvp = parse_uuid(shooter).is_some_and(|a| pvp_blocked(ess, host, player, a));
            by_type || pvp
        }
        ("fall", _) => guarded(ess, host, disable(ess, "fall"), "fall"),
        ("suffocation", _) => guarded(ess, host, disable(ess, "suffocate"), "suffocation"),
        ("fire" | "fire_tick", _) => guarded(ess, host, disable(ess, "firedmg"), "fire"),
        ("drowning", _) => guarded(ess, host, disable(ess, "drown"), "drowning"),
        ("lightning", _) => guarded(ess, host, disable(ess, "lightning"), "lightning"),
        ("wither", _) => guarded(ess, host, disable(ess, "wither"), "wither"),
        _ => false,
    };
    if blocked { Allow::No } else { Allow::Yes }
}

/// The protect decision for a world event (true = cancel).
pub fn world_blocked(ess: &mut Ess, host: &mut dyn Host, kind: &str, world: &str, pos: [i32; 3]) -> bool {
    let (head, tail) = kind.split_once(':').unwrap_or((kind, ""));
    match (head, tail) {
        ("fire_spread", _) | ("ignite", "spread") | ("burn", _) => prevent(ess, "fire-spread"),
        ("ignite", "flint_and_steel") => {
            let obsidian = |s: Option<String>| s.is_some_and(|b| b.starts_with("minecraft:obsidian"));
            if obsidian(host.block(world, pos)) || obsidian(host.block(world, [pos[0], pos[1] - 1, pos[2]])) {
                prevent(ess, "portal-creation")
            } else {
                prevent(ess, "flint-fire")
            }
        }
        ("ignite", "lava") => prevent(ess, "lava-fire-spread"),
        ("ignite", "lightning") => prevent(ess, "lightning-fire-spread"),
        ("ignite", "fireball") => prevent(ess, "fireball-fire"),
        ("portal", _) => prevent(ess, "portal-creation"),
        ("flow", what) => {
            let what = if what.is_empty() {
                let b = host.block(world, pos).unwrap_or_default();
                if b.contains("water") {
                    "water"
                } else if b.contains("lava") {
                    "lava"
                } else {
                    "air"
                }
            } else {
                what
            };
            match what {
                "water" => prevent(ess, "water-flow"),
                "lava" => prevent(ess, "lava-flow"),
                _ => prevent(ess, "water-bucket-flow"),
            }
        }
        ("explode", entity) => match entity {
            "ender_dragon" => prevent(ess, "enderdragon-blockdamage"),
            "wither" => prevent(ess, "wither-spawnexplosion"),
            "creeper" => {
                let max = ess.config_i64("protect.creeper.max-height", -1);
                prevent(ess, "creeper-explosion") || prevent(ess, "creeper-blockdamage") || (max >= 0 && pos[1] > max as i32)
            }
            "tnt" => prevent(ess, "tnt-explosion"),
            "fireball" | "small_fireball" => prevent(ess, "fireball-explosion"),
            "wind_charge" | "breeze_wind_charge" => prevent(ess, "windcharge-explosion"),
            "wither_skull" => prevent(ess, "witherskull-explosion"),
            "tnt_minecart" => prevent(ess, "tnt-minecart-explosion"),
            "end_crystal" => prevent(ess, "ender-crystal-explosion"),
            "bed" => prevent(ess, "bed-explosion"),
            "respawn_anchor" => prevent(ess, "respawn-anchor-explosion"),
            _ => false,
        },
        ("bed_explode", _) => prevent(ess, "bed-explosion"),
        ("spawn", entity) => {
            let name = entity.trim_start_matches("minecraft:");
            !name.is_empty() && name != "player" && ess.config_bool(&format!("protect.prevent.spawn.{name}"), false)
        }
        ("change_block", "enderman") => prevent(ess, "enderman-pickup"),
        ("change_block", "wither") => prevent(ess, "wither-blockreplace"),
        ("change_block", "sheep") => prevent(ess, "sheep-eat-grass"),
        ("change_block", "ravager") => prevent(ess, "ravager-thief"),
        ("door_break", "zombie") => prevent(ess, "zombie-door-break"),
        ("weather", "storm") => disable(ess, "weather.storm"),
        ("weather", "thunder") => disable(ess, "weather.thunder"),
        ("lightning", _) => disable(ess, "weather.lightning"),
        ("transform", what) => ess.config_bool(&format!("protect.prevent.transformation.{what}"), false),
        ("villager_damage", _) => prevent(ess, "villager-death"),
        ("item_damage", cause) => match cause {
            "lava" | "fire" | "fire_tick" => disable(ess, "lava-itemdamage") || prevent(ess, "lava-itemdamage"),
            "tnt" => prevent(ess, "tnt-itemdamage"),
            "creeper" => prevent(ess, "creeper-itemdamage"),
            "tnt_minecart" => prevent(ess, "tnt-minecart-itemdamage"),
            "wither_skull" => prevent(ess, "witherskull-itemdamage"),
            "fireball" | "small_fireball" => prevent(ess, "fireball-itemdamage"),
            _ => false,
        },
        ("piston", block) => in_list(ess, "protect.blacklist.piston", &material(block)),
        ("dispense", item) => in_list(ess, "protect.blacklist.dispenser", &material(item)),
        ("target", who) => {
            prevent(ess, "entitytarget") && parse_uuid(who).is_some_and(|p| !ess.has(host, Sender::Player(p), "essentials.protect.entitytarget.bypass"))
        }
        _ => false,
    }
}

fn on_world(ess: &mut Ess, host: &mut dyn Host, kind: &str, world: &str, pos: [i32; 3]) -> Allow {
    if world_blocked(ess, host, kind, world, pos) { Allow::No } else { Allow::Yes }
}

// ===== AntiBuild =====================================================================================

/// Bukkit's Material name for an id or block state (`minecraft:lava_bucket` -> `LAVA_BUCKET`).
pub fn material(id: &str) -> String {
    let id = id.split('[').next().unwrap_or(id);
    id.trim_start_matches("minecraft:").to_uppercase()
}

/// EssentialsAntiBuild.getNameForType: `lava bucket`.
fn name_for_type(material: &str) -> String {
    material.to_lowercase().replace('_', " ")
}

/// A config list given as a YAML list or a comma separated string (`LAVA,TNT,LAVA_BUCKET`).
fn protect_list(ess: &Ess, path: &str) -> Vec<String> {
    ess.config_list(path).iter().flat_map(|s| s.split(',').map(|p| p.trim().to_uppercase()).collect::<Vec<_>>()).filter(|s| !s.is_empty()).collect()
}

fn in_list(ess: &Ess, path: &str, material: &str) -> bool {
    protect_list(ess, path).iter().any(|m| m == material)
}

/// User.canBuild: operators, or `essentials.build`.
fn can_build(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> bool {
    host.op_level(player) > 0 || ess.has(host, Sender::Player(player), "essentials.build")
}

fn meta_perm(ess: &mut Ess, host: &mut dyn Host, player: Uuid, action: &str, material: &str) -> bool {
    ess.has(host, Sender::Player(player), &format!("essentials.build.{action}.{material}"))
}

fn warn(ess: &mut Ess, host: &mut dyn Host, player: Uuid, key: &str, what: String) {
    if ess.config_bool("protect.disable.warn-on-build-disallow", false) {
        ess.send_tl(host, Sender::Player(player), key, &[Arg::Plain(what)]);
    }
}

/// EssentialsConnect.alert: the console and every player with essentials.protect.alerts.
fn alert(ess: &mut Ess, host: &mut dyn Host, player: Uuid, item: &str, key: &str) {
    let name = host.name(player).unwrap_or_default();
    let at = host.location(player).unwrap_or_else(|| Loc::new("minecraft:overworld", 0.0, 0.0, 0.0));
    let [x, y, z] = at.block();
    let place = format!("{},{x},{y},{z}", crate::game::ess_world::world_name(&at.world));
    let args = [Arg::Plain(name), Arg::Parsed(ess.tl(key, &[])), Arg::Plain(item.to_string()), Arg::Plain(place)];
    ess.send_tl(host, Sender::Console, "alertFormat", &args);
    for p in host.online() {
        if ess.has(host, Sender::Player(p), "essentials.protect.alerts") {
            ess.send_tl(host, Sender::Player(p), "alertFormat", &args);
        }
    }
}

fn build_disabled(ess: &Ess) -> bool {
    ess.config_bool("protect.disable.build", true)
}

fn use_disabled(ess: &Ess) -> bool {
    ess.config_bool("protect.disable.use", true)
}

fn on_place(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _world: &str, _pos: [i32; 3], state: &str) -> Allow {
    let mat = material(state);
    if build_disabled(ess) && !can_build(ess, host, player) && !meta_perm(ess, host, player, "place", &mat) {
        warn(ess, host, player, "antiBuildPlace", name_for_type(&mat));
        return Allow::No;
    }
    if in_list(ess, "protect.blacklist.placement", &mat) && !ess.has(host, Sender::Player(player), "essentials.protect.exemptplacement") {
        warn(ess, host, player, "antiBuildPlace", name_for_type(&mat));
        return Allow::No;
    }
    if in_list(ess, "protect.alert.on-placement", &mat) && !ess.has(host, Sender::Player(player), "essentials.protect.alerts.notrigger") {
        alert(ess, host, player, &name_for_type(&mat), "alertPlaced");
    }
    Allow::Yes
}

/// A standing sign on top of the block (or a wall sign facing away from it) the player may not break.
fn breaks_protected_sign(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3]) -> bool {
    let mut attached = Vec::new();
    if let Some(b) = host.block(world, [pos[0], pos[1] + 1, pos[2]]) {
        if b.contains("_sign") && !b.contains("wall") && !b.contains("hanging") {
            attached.push(b);
        }
    }
    if let Some(b) = host.block(world, [pos[0], pos[1] - 1, pos[2]]) {
        if b.contains("_hanging_sign") && !b.contains("wall") {
            attached.push(b);
        }
    }
    for (dx, dz, facing) in [(0, -1, "north"), (1, 0, "east"), (0, 1, "south"), (-1, 0, "west")] {
        if let Some(b) = host.block(world, [pos[0] + dx, pos[1], pos[2] + dz]) {
            if b.contains("wall_sign") || b.contains("wall_hanging_sign") {
                if b.contains(&format!("facing={facing}")) {
                    attached.push(b);
                }
            }
        }
    }
    for sign in attached {
        let mat = material(&sign);
        if build_disabled(ess) && !can_build(ess, host, player) && !meta_perm(ess, host, player, "break", &mat) {
            return true;
        }
        if in_list(ess, "protect.blacklist.break", &mat) && !ess.has(host, Sender::Player(player), "essentials.protect.exemptbreak") {
            return true;
        }
    }
    false
}

fn on_break(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], state: &str) -> Allow {
    let mat = material(state);
    if breaks_protected_sign(ess, host, player, world, pos) {
        warn(ess, host, player, "antiBuildBreak", name_for_type(&mat));
        return Allow::No;
    }
    if build_disabled(ess) && !can_build(ess, host, player) && !meta_perm(ess, host, player, "break", &mat) {
        warn(ess, host, player, "antiBuildBreak", name_for_type(&mat));
        return Allow::No;
    }
    if in_list(ess, "protect.blacklist.break", &mat) && !ess.has(host, Sender::Player(player), "essentials.protect.exemptbreak") {
        warn(ess, host, player, "antiBuildBreak", name_for_type(&mat));
        return Allow::No;
    }
    if in_list(ess, "protect.alert.on-break", &mat) && !ess.has(host, Sender::Player(player), "essentials.protect.alerts.notrigger") {
        alert(ess, host, player, &name_for_type(&mat), "alertBroke");
    }
    Allow::Yes
}

fn on_interact(ess: &mut Ess, host: &mut dyn Host, player: Uuid, block: Option<(&str, [i32; 3], &str)>, held: Option<&Item>) -> Allow {
    let mut out = Allow::Yes;
    let who = Sender::Player(player);
    if let Some(item) = held {
        let mat = material(&item.id);
        if in_list(ess, "protect.blacklist.usage", &mat) && !ess.has(host, who, "essentials.protect.exemptusage") {
            warn(ess, host, player, "antiBuildUse", mat.clone());
            return Allow::No;
        }
        if in_list(ess, "protect.alert.on-use", &mat) && !ess.has(host, who, "essentials.protect.alerts.notrigger") {
            alert(ess, host, player, &mat, "alertUsed");
        }
    }
    if use_disabled(ess) && !can_build(ess, host, player) {
        if let Some(item) = held {
            let mat = material(&item.id);
            if !meta_perm(ess, host, player, "interact", &mat) {
                out = Allow::No;
                warn(ess, host, player, "antiBuildUse", mat);
            }
        }
        if let Some((_, _, state)) = block {
            let mat = material(state);
            if !meta_perm(ess, host, player, "interact", &mat) {
                out = Allow::No;
                warn(ess, host, player, "antiBuildInteract", mat);
            }
        }
    }
    out
}

/// AntiBuild's PlayerDropItemEvent (the glue calls it; the core has no drop event).
pub fn on_drop(ess: &mut Ess, host: &mut dyn Host, player: Uuid, item: &Item) -> Allow {
    let mat = material(&item.id);
    if use_disabled(ess) && !can_build(ess, host, player) && !meta_perm(ess, host, player, "drop", &mat) {
        warn(ess, host, player, "antiBuildDrop", mat);
        return Allow::No;
    }
    Allow::Yes
}

/// AntiBuild's pickup listener (silent, as EssentialsX).
pub fn on_pickup(ess: &mut Ess, host: &mut dyn Host, player: Uuid, item: &Item) -> Allow {
    let mat = material(&item.id);
    if use_disabled(ess) && !can_build(ess, host, player) && !meta_perm(ess, host, player, "pickup", &mat) {
        return Allow::No;
    }
    Allow::Yes
}

/// AntiBuild's CraftItemEvent.
pub fn on_craft(ess: &mut Ess, host: &mut dyn Host, player: Uuid, result: &Item) -> Allow {
    let mat = material(&result.id);
    if use_disabled(ess) && !can_build(ess, host, player) && !meta_perm(ess, host, player, "craft", &mat) {
        warn(ess, host, player, "antiBuildCraft", mat);
        return Allow::No;
    }
    Allow::Yes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ess_testkit::*;

    fn with(nodes: &[&str], config: &str) -> (TestServer, Uuid, Uuid) {
        let mut t = TestServer::with_config(config);
        let list = nodes.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(", ");
        t.permissions(&format!("groups:\n  default:\n    default: true\n    permissions: [{list}]\n"));
        let carol = t.join("Carol");
        let admin = t.join("Admin");
        t.op("Admin");
        (t, carol, admin)
    }

    const W: &str = "minecraft:overworld";

    #[test]
    fn damage_types_follow_disable_and_permissions() {
        let (mut t, carol, admin) = with(&[], "protect:\n  disable:\n    fall: true\n    drown: true\n    firedmg: true");
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "fall", 3.0), Allow::No);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "drowning", 1.0), Allow::No);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "fire_tick", 1.0), Allow::No);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "lava", 4.0), Allow::Yes);
        // Ops hold essentials.protect.damage.disable: still protected.
        assert_eq!(t.ess.on_damage(&mut t.host, admin, "fall", 3.0), Allow::No);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.protect.damage.fall]\n");
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "fall", 3.0), Allow::Yes);
        let (mut t, carol, _) = with(&[], "");
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "fall", 3.0), Allow::Yes);
    }

    #[test]
    fn explosions_and_projectiles_to_players() {
        let (mut t, carol, _) = with(&[], "protect:\n  prevent:\n    creeper-playerdamage: true\n    tnt-playerdamage: true\n  disable:\n    projectiles: true");
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "explosion:creeper", 9.0), Allow::No);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "explosion:tnt", 9.0), Allow::No);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "explosion:fireball", 9.0), Allow::Yes);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, "projectile:-", 2.0), Allow::No);
    }

    #[test]
    fn pvp_needs_protect_pvp_on_both_sides() {
        let (mut t, carol, _) = with(&[], "protect:\n  disable:\n    pvp: true");
        let dave = t.join("Dave");
        let by_dave = format!("player:{}", uuid_string(dave));
        assert_eq!(t.ess.on_damage(&mut t.host, carol, &by_dave, 2.0), Allow::No);
        assert_eq!(t.ess.on_damage(&mut t.host, carol, &format!("projectile:{}", uuid_string(dave)), 2.0), Allow::No);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.protect.pvp]\n");
        assert_eq!(t.ess.on_damage(&mut t.host, carol, &by_dave, 2.0), Allow::Yes);
    }

    #[test]
    fn world_events() {
        let (mut t, carol, _) = with(&[], "");
        let on = |t: &mut TestServer, kind: &str| t.ess.on_world(&mut t.host, kind, W, [0, 64, 0]);
        // Defaults: fire spread and ender dragon block damage are prevented.
        assert_eq!(on(&mut t, "ignite:spread"), Allow::No);
        assert_eq!(on(&mut t, "fire_spread"), Allow::No);
        assert_eq!(on(&mut t, "burn"), Allow::No);
        assert_eq!(on(&mut t, "ignite:lava"), Allow::No);
        assert_eq!(on(&mut t, "ignite:lightning"), Allow::No);
        assert_eq!(on(&mut t, "explode:ender_dragon"), Allow::No);
        assert_eq!(on(&mut t, "ignite:flint_and_steel"), Allow::Yes);
        assert_eq!(on(&mut t, "explode:creeper"), Allow::Yes);
        assert_eq!(on(&mut t, "explode:tnt"), Allow::Yes);
        assert_eq!(on(&mut t, "flow:water"), Allow::Yes);
        assert_eq!(on(&mut t, "spawn:minecraft:zombie"), Allow::Yes);
        assert_eq!(on(&mut t, "weather:storm"), Allow::Yes);
        let target = format!("target:{}", uuid_string(carol));
        assert_eq!(on(&mut t, &target), Allow::Yes);
        let (mut t, carol, admin) = with(
            &[],
            "protect:\n  prevent:\n    water-flow: true\n    creeper-explosion: true\n    tnt-explosion: true\n    flint-fire: true\n    portal-creation: true\n    entitytarget: true\n    enderman-pickup: true\n    zombie-door-break: true\n    transformation:\n      witch: true\n    spawn:\n      zombie: true\n  blacklist:\n    piston: OBSIDIAN\n  disable:\n    weather:\n      storm: true\n      lightning: true",
        );
        let on = |t: &mut TestServer, kind: &str| t.ess.on_world(&mut t.host, kind, W, [0, 64, 0]);
        assert_eq!(on(&mut t, "flow:water"), Allow::No);
        assert_eq!(on(&mut t, "flow:lava"), Allow::Yes);
        t.host.blocks.insert((W.into(), [0, 64, 0]), "minecraft:water[level=0]".into());
        assert_eq!(on(&mut t, "flow"), Allow::No);
        assert_eq!(on(&mut t, "explode:creeper"), Allow::No);
        assert_eq!(on(&mut t, "explode:tnt"), Allow::No);
        assert_eq!(on(&mut t, "explode:end_crystal"), Allow::Yes);
        assert_eq!(on(&mut t, "ignite:flint_and_steel"), Allow::No);
        assert_eq!(on(&mut t, "portal"), Allow::No);
        assert_eq!(on(&mut t, "spawn:minecraft:zombie"), Allow::No);
        assert_eq!(on(&mut t, "spawn:minecraft:skeleton"), Allow::Yes);
        assert_eq!(on(&mut t, "weather:storm"), Allow::No);
        assert_eq!(on(&mut t, "weather:thunder"), Allow::Yes);
        assert_eq!(on(&mut t, "lightning"), Allow::No);
        assert_eq!(on(&mut t, "change_block:enderman"), Allow::No);
        assert_eq!(on(&mut t, "door_break:zombie"), Allow::No);
        assert_eq!(on(&mut t, "transform:witch"), Allow::No);
        assert_eq!(on(&mut t, "piston:minecraft:obsidian"), Allow::No);
        assert_eq!(on(&mut t, "piston:minecraft:stone"), Allow::Yes);
        let carol_t = format!("target:{}", uuid_string(carol));
        let admin_t = format!("target:{}", uuid_string(admin));
        assert_eq!(on(&mut t, &carol_t), Allow::No);
        assert_eq!(on(&mut t, &admin_t), Allow::Yes);
        // Creeper max height.
        let (mut t, _, _) = with(&[], "protect:\n  creeper:\n    max-height: 50");
        assert_eq!(t.ess.on_world(&mut t.host, "explode:creeper", W, [0, 64, 0]), Allow::No);
        assert_eq!(t.ess.on_world(&mut t.host, "explode:creeper", W, [0, 40, 0]), Allow::Yes);
    }

    #[test]
    fn antibuild_place_needs_build_or_meta_permission() {
        let (mut t, carol, admin) = with(&[], "");
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:stone"), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to place stone here."]);
        assert_eq!(t.ess.on_block_place(&mut t.host, admin, W, [0, 64, 0], "minecraft:stone"), Allow::Yes);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.build.place.stone]\n");
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:stone"), Allow::Yes);
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:oak_planks"), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to place oak planks here."]);
        // build: false turns the check off.
        let (mut t, carol, _) = with(&[], "protect:\n  disable:\n    build: false\n    warn-on-build-disallow: false");
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:stone"), Allow::Yes);
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:tnt"), Allow::No);
        assert!(t.said(carol).is_empty());
    }

    #[test]
    fn antibuild_blacklist_and_alerts() {
        let (mut t, carol, admin) = with(&["essentials.build"], "");
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:tnt"), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to place tnt here."]);
        t.said(admin);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.build, essentials.protect.exemptplacement]\n");
        assert_eq!(t.ess.on_block_place(&mut t.host, carol, W, [0, 64, 0], "minecraft:tnt"), Allow::Yes);
        assert_eq!(t.said(admin), ["[Carol] placed: tnt at: world,0,64,0"]);
        assert!(t.said(carol).is_empty());
        assert_eq!(t.host.said(Sender::Console), ["[Carol] placed: tnt at: world,0,64,0"]);
        // Breaking: blacklist and alert lists as YAML lists too.
        let (mut t, carol, admin) = with(&["essentials.build"], "protect:\n  blacklist:\n    break: [BEDROCK]\n  alert:\n    on-break: [DIAMOND_ORE]");
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, -64, 0], "minecraft:bedrock"), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to break bedrock blocks here."]);
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, 10, 0], "minecraft:diamond_ore"), Allow::Yes);
        assert_eq!(t.said(admin), ["[Carol] broke: diamond ore at: world,0,64,0"]);
    }

    #[test]
    fn antibuild_break_and_attached_signs() {
        let (mut t, carol, _) = with(&["essentials.build.break.stone"], "");
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, 63, 0], "minecraft:stone"), Allow::Yes);
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, 63, 0], "minecraft:dirt"), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to break dirt blocks here."]);
        t.host.blocks.insert((W.into(), [0, 64, 0]), "minecraft:oak_sign[rotation=0]".into());
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, 63, 0], "minecraft:stone"), Allow::No);
        t.host.blocks.insert((W.into(), [0, 64, 0]), "minecraft:air".into());
        t.host.blocks.insert((W.into(), [1, 63, 0]), "minecraft:oak_wall_sign[facing=east]".into());
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, 63, 0], "minecraft:stone"), Allow::No);
        t.host.blocks.insert((W.into(), [1, 63, 0]), "minecraft:oak_wall_sign[facing=west]".into());
        assert_eq!(t.ess.on_block_break(&mut t.host, carol, W, [0, 63, 0], "minecraft:stone"), Allow::Yes);
    }

    #[test]
    fn antibuild_use_interact_drop_pickup_craft() {
        let (mut t, carol, admin) = with(&["essentials.build"], "");
        let lava = Item::new("lava_bucket", 1);
        assert_eq!(t.ess.on_interact(&mut t.host, carol, None, Some(&lava)), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to use LAVA_BUCKET."]);
        t.said(admin);
        // Ops are exempt and (alerts.notrigger) don't raise alerts.
        assert_eq!(t.ess.on_interact(&mut t.host, admin, None, Some(&lava)), Allow::Yes);
        assert!(t.said(admin).is_empty());
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.build, essentials.protect.exemptusage]\n");
        assert_eq!(t.ess.on_interact(&mut t.host, carol, None, Some(&lava)), Allow::Yes);
        assert_eq!(t.said(admin), ["[Carol] used: LAVA_BUCKET at: world,0,64,0"]);
        // Without build: interact checks the held item and the block.
        let (mut t, carol, _) = with(&["essentials.build.interact.stick"], "");
        let stick = Item::new("stick", 1);
        assert_eq!(t.ess.on_interact(&mut t.host, carol, None, Some(&stick)), Allow::Yes);
        assert_eq!(t.ess.on_interact(&mut t.host, carol, Some((W, [0, 64, 0], "minecraft:chest[facing=north]")), Some(&stick)), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to interact with CHEST."]);
        let bread = Item::new("bread", 1);
        assert_eq!(on_drop(&mut t.ess, &mut t.host, carol, &bread), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to drop BREAD."]);
        assert_eq!(on_pickup(&mut t.ess, &mut t.host, carol, &bread), Allow::No);
        assert!(t.said(carol).is_empty());
        assert_eq!(on_craft(&mut t.ess, &mut t.host, carol, &bread), Allow::No);
        assert_eq!(t.said(carol), ["You are not permitted to create BREAD."]);
        assert_eq!(on_drop(&mut t.ess, &mut t.host, carol, &stick), Allow::No);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.build.drop.stick, essentials.build.pickup.bread]\n");
        assert_eq!(on_drop(&mut t.ess, &mut t.host, carol, &stick), Allow::Yes);
        assert_eq!(on_pickup(&mut t.ess, &mut t.host, carol, &bread), Allow::Yes);
    }
}
