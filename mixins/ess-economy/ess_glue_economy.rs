// horizon-essentials glue for area `economy` (game only): `crate::game::ess_glue_economy`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn economy_sign_lines(&mut self, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
        None
    }
    pub(super) fn economy_set_sign_lines(&mut self, world: &str, pos: [i32; 3], lines: &[String; 4]) -> bool {
        false
    }
}
