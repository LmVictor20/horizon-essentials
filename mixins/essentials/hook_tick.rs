// Runs first in native_tick, once per server tick.
fn ess_tick(game: &mut crate::game::Game<'_>) {
    crate::game::ess_glue::on_tick(game);
}
