// horizon-essentials glue for area `admin` (game only): `crate::game::ess_glue_admin`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn admin_lookup(&mut self, name: &str) -> Option<(Uuid, String)> {
        None
    }
    pub(super) fn admin_set_hidden(&mut self, player: Uuid, hidden: bool, seers: &[Uuid]) {
    }
    pub(super) fn admin_bans(&mut self, ip: bool) -> Vec<Ban> {
        Vec::new()
    }
    pub(super) fn admin_ban(&mut self, ip: bool, ban: &Ban) {
    }
    pub(super) fn admin_unban(&mut self, ip: bool, target: &str) -> bool {
        false
    }
    pub(super) fn admin_stats(&mut self) -> ServerStats {
        ServerStats::default()
    }
    pub(super) fn admin_max_players(&mut self) -> usize {
        20
    }
    pub(super) fn admin_statistic(&mut self, player: Uuid, stat: &str) -> i64 {
        0
    }
}
