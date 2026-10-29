// horizon-essentials glue for area `items` (game only): `crate::game::ess_glue_items`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn items_inventory(&mut self, player: Uuid) -> Vec<Option<Item>> {
        vec![None; 41]
    }
    pub(super) fn items_set_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
    }
    pub(super) fn items_held_slot(&mut self, player: Uuid) -> usize {
        0
    }
    pub(super) fn items_ender_chest(&mut self, player: Uuid) -> Vec<Option<Item>> {
        vec![None; 27]
    }
    pub(super) fn items_set_ender_slot(&mut self, player: Uuid, slot: usize, item: Option<Item>) {
    }
    pub(super) fn items_open_container(&mut self, player: Uuid, container: &Container) -> bool {
        false
    }
    pub(super) fn items_open_station(&mut self, player: Uuid, station: Station) -> bool {
        false
    }
    pub(super) fn items_close_container(&mut self, player: Uuid) {
    }
    pub(super) fn items_drop_item(&mut self, at: &Loc, item: &Item) {
    }
    pub(super) fn items_item_ids(&mut self) -> Vec<String> {
        Vec::new()
    }
    pub(super) fn items_max_stack(&mut self, item: &str) -> i32 {
        64
    }
    pub(super) fn items_max_damage(&mut self, item: &str) -> i32 {
        0
    }
    pub(super) fn items_enchantments(&mut self) -> Vec<(String, i32)> {
        Vec::new()
    }
}
