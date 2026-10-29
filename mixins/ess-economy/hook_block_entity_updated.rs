// Runs first in mark_block_entity_updated: notices finished sign edits (their sign_change event runs next tick).
fn ess_economy_block_entity_updated(game: &mut crate::game::Game<'_>, key: (i32, i32, i32)) {
    crate::game::ess_glue_economy::on_block_entity_updated(game, key);
}
