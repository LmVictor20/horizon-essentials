// horizon-essentials glue for area `tp` (game only): `crate::game::ess_glue_tp`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn tp_world_spawn(&mut self, world: &str) -> Option<Loc> {
        None
    }
    pub(super) fn tp_set_world_spawn(&mut self, at: &Loc) {
    }
    pub(super) fn tp_bed_spawn(&mut self, player: Uuid) -> Option<Loc> {
        None
    }
    pub(super) fn tp_block(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        None
    }
    pub(super) fn tp_set_block(&mut self, world: &str, pos: [i32; 3], state: &str) -> bool {
        false
    }
    pub(super) fn tp_highest_y(&mut self, world: &str, x: i32, z: i32) -> Option<i32> {
        None
    }
    pub(super) fn tp_height_range(&mut self, world: &str) -> (i32, i32) {
        (-64, 320)
    }
    pub(super) fn tp_target_block(&mut self, player: Uuid, max: f64) -> Option<[i32; 3]> {
        None
    }
    pub(super) fn tp_biome(&mut self, world: &str, pos: [i32; 3]) -> Option<String> {
        None
    }
}
