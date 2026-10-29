// horizon-essentials glue for area `social` (game only): `crate::game::ess_glue_social`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn social_broadcast(&mut self, text: &[Span]) {
        for p in self.online() {
            self.send(Sender::Player(p), text);
        }
        self.send(Sender::Console, text);
    }
    pub(super) fn social_chat_as(&mut self, player: Uuid, text: &[Span]) {
        self.social_broadcast(text);
    }
    pub(super) fn social_set_display_name(&mut self, player: Uuid, name: Option<&[Span]>) {
    }
    pub(super) fn social_set_tab_name(&mut self, player: Uuid, name: Option<&[Span]>) {
    }
    pub(super) fn social_set_motd(&mut self, motd: &[Span]) {
    }
}
