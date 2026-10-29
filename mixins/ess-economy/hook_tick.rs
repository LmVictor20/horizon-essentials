// Runs in native_tick after the core's hook: player snapshot, queued messages, last tick's sign edits.
fn ess_economy_tick(game: &mut crate::game::Game<'_>) {
    crate::game::ess_glue_economy::on_tick(game);
}
