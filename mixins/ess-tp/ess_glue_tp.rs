// horizon-essentials glue for area `tp` (game only): `crate::game::ess_glue_tp`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here). Deaths and
// respawns need no hook: the tp area polls Host::is_dead (Guest.dead) every tick (Game::guest_die and
// guest_respawn hold macros, which HEAD injections refuse).
//
// Blocks are read from the level in place (`game.world`) or from a parked level's chunks (`game.levels`);
// a chunk that is not loaded answers None. Writes go through vanilla's /setblock (any dimension).

use super::ess_glue::GameHost;
use crate::game::ess_core::*;
use horizon_world::registry::BlockStateId;
use std::sync::Arc;

impl GameHost<'_, '_> {
    /// The loaded chunk holding (x, z) in a dimension.
    fn tp_chunk(&mut self, world: &str, x: i32, z: i32) -> Option<Arc<horizon_client::chunk::Chunk>> {
        let key = (x >> 4, z >> 4);
        if self.game.dim.id() == world {
            return self.game.world.chunks.get(&key).cloned();
        }
        let dim = super::Dim::from_id(world);
        if dim.id() != world {
            return None;
        }
        self.game.levels.parked_chunks().find(|(d, _)| *d == dim).and_then(|(_, chunks)| chunks.get(&key).cloned())
    }

    /// `minecraft:oak_log[axis=y]`.
    fn tp_state_string(&self, state: BlockStateId) -> String {
        let Some(s) = self.game.pack.assets.registry.state(state) else {
            return "minecraft:air".into();
        };
        if s.properties.is_empty() {
            return s.name.clone();
        }
        let props: Vec<String> = s.properties.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("{}[{}]", s.name, props.join(","))
    }

    pub(super) fn tp_world_spawn(&mut self, world: &str) -> Option<Loc> {
        if world != "minecraft:overworld" {
            return None;
        }
        let [x, y, z] = self.game.level.world_spawn?;
        Some(Loc::new(world, f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5))
    }
    pub(super) fn tp_set_world_spawn(&mut self, at: &Loc) {
        let b = at.block();
        self.console_command(&format!("execute in {} run setworldspawn {} {} {} {}", at.world, b[0], b[1], b[2], at.yaw));
    }
    pub(super) fn tp_bed_spawn(&mut self, player: Uuid) -> Option<Loc> {
        let g = self.guest(player)?;
        if let Some([x, y, z]) = g.spawn {
            return Some(Loc { world: "minecraft:overworld".into(), x, y, z, yaw: g.yaw, pitch: 0.0 });
        }
        let (dim, pos, yaw, _) = g.respawn_elsewhere.clone()?;
        Some(Loc { world: dim, x: f64::from(pos[0]) + 0.5, y: f64::from(pos[1]), z: f64::from(pos[2]) + 0.5, yaw, pitch: 0.0 })
    }
    pub(super) fn tp_block(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        let chunk = self.tp_chunk(world, pos[0], pos[2])?;
        let (min, max) = self.tp_height_range(world);
        if pos[1] < min || pos[1] >= max {
            return Some("minecraft:void_air".into());
        }
        Some(self.tp_state_string(chunk.block(pos[0] & 15, pos[1], pos[2] & 15)))
    }
    pub(super) fn tp_set_block(&mut self, world: &str, pos: [i32; 3], state: &str) -> bool {
        self.console_command(&format!("execute in {world} run setblock {} {} {} {state}", pos[0], pos[1], pos[2])).0
    }
    pub(super) fn tp_highest_y(&mut self, world: &str, x: i32, z: i32) -> Option<i32> {
        let chunk = self.tp_chunk(world, x, z)?;
        let (min, max) = self.tp_height_range(world);
        let registry = self.game.pack.assets.registry;
        (min..max).rev().find(|y| {
            let state = chunk.block(x & 15, *y, z & 15);
            registry.state(state).is_some_and(|s| !s.name.ends_with("air"))
        })
    }
    pub(super) fn tp_height_range(&mut self, world: &str) -> (i32, i32) {
        match world {
            "minecraft:the_nether" | "minecraft:the_end" => (0, 256),
            _ => (-64, 320),
        }
    }
    /// The first block that is not see-through along the player's look, from the eyes (Bukkit getTargetBlock).
    pub(super) fn tp_target_block(&mut self, player: Uuid, max: f64) -> Option<[i32; 3]> {
        let g = self.guest(player)?;
        let world = g.dimension.clone();
        let eye = [g.position[0], g.position[1] + g.eye_height(), g.position[2]];
        let (yaw, pitch) = (f64::from(g.yaw).to_radians(), f64::from(g.pitch).to_radians());
        let dir = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
        let mut last = None;
        let steps = (max * 20.0) as i32;
        for i in 0..=steps {
            let t = f64::from(i) * 0.05;
            let pos = [(eye[0] + dir[0] * t).floor() as i32, (eye[1] + dir[1] * t).floor() as i32, (eye[2] + dir[2] * t).floor() as i32];
            if last == Some(pos) {
                continue;
            }
            last = Some(pos);
            let state = self.tp_block(&world, pos)?;
            let id = state.split('[').next().unwrap_or("");
            let id = id.strip_prefix("minecraft:").unwrap_or(id);
            if !(crate::game::ess_tp::passable(id) || id == "water") {
                return Some(pos);
            }
        }
        None
    }
    /// On the death screen (Guest.dead): the tp area polls it for death and respawn.
    pub(super) fn tp_is_dead(&mut self, player: Uuid) -> bool {
        self.guest(player).is_some_and(|g| g.dead)
    }
    pub(super) fn tp_biome(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        let chunk = self.tp_chunk(world, pos[0], pos[2])?;
        let biomes = self.game.res.biomes();
        biomes.list.get(chunk.biome(pos[0], pos[1], pos[2]) as usize).map(|b| b.name.clone())
    }
}
