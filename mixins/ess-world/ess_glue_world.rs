// horizon-essentials glue for area `world` (game only): `crate::game::ess_glue_world`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn world_game_mode(&mut self, player: Uuid) -> GameMode {
        GameMode::Survival
    }
    pub(super) fn world_set_game_mode(&mut self, player: Uuid, mode: GameMode) {
    }
    pub(super) fn world_health(&mut self, player: Uuid) -> (f64, f64) {
        (20.0, 20.0)
    }
    pub(super) fn world_set_health(&mut self, player: Uuid, health: f64) {
    }
    pub(super) fn world_food(&mut self, player: Uuid) -> (i32, f32) {
        (20, 5.0)
    }
    pub(super) fn world_set_food(&mut self, player: Uuid, food: i32, saturation: f32) {
    }
    pub(super) fn world_set_fire_ticks(&mut self, player: Uuid, ticks: i32) {
    }
    pub(super) fn world_fire_ticks(&mut self, player: Uuid) -> i32 {
        0
    }
    pub(super) fn world_set_freeze_ticks(&mut self, player: Uuid, ticks: i32) {
    }
    pub(super) fn world_exp(&mut self, player: Uuid) -> i64 {
        0
    }
    pub(super) fn world_set_exp(&mut self, player: Uuid, total: i64) {
    }
    pub(super) fn world_flying(&mut self, player: Uuid) -> bool {
        false
    }
    pub(super) fn world_set_allow_flight(&mut self, player: Uuid, allow: bool) {
    }
    pub(super) fn world_allow_flight(&mut self, player: Uuid) -> bool {
        false
    }
    pub(super) fn world_set_flying(&mut self, player: Uuid, flying: bool) {
    }
    pub(super) fn world_set_fly_speed(&mut self, player: Uuid, speed: f32) {
    }
    pub(super) fn world_set_walk_speed(&mut self, player: Uuid, speed: f32) {
    }
    pub(super) fn world_set_invulnerable(&mut self, player: Uuid, invulnerable: bool) {
    }
    pub(super) fn world_set_player_time(&mut self, player: Uuid, time: Option<(i64, bool)>) {
    }
    pub(super) fn world_set_player_weather(&mut self, player: Uuid, downfall: Option<bool>) {
    }
    pub(super) fn world_entity_types(&mut self) -> Vec<String> {
        Vec::new()
    }
    pub(super) fn world_world_time(&mut self, world: &str) -> i64 {
        0
    }
    pub(super) fn world_set_world_time(&mut self, world: &str, time: i64) {
    }
    pub(super) fn world_weather(&mut self, world: &str) -> (bool, bool) {
        (false, false)
    }
    pub(super) fn world_set_weather(&mut self, world: &str, rain: bool, thunder: bool, duration: Option<i32>) {
    }
    pub(super) fn world_spawn_entity(&mut self, kind: &str, at: &Loc, snbt: Option<&str>) -> bool {
        false
    }
    pub(super) fn world_strike_lightning(&mut self, at: &Loc, effect_only: bool) {
    }
    pub(super) fn world_remove_entities(&mut self, world: &str, center: Option<&Loc>, radius: Option<f64>, kinds: &[String]) -> usize {
        0
    }
}
