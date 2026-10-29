// Once per server tick, after the core's tick hook: held chat, command tree, join/quit line flags.
fn ess_social_tick(game: &mut crate::game::Game<'_>) {
    crate::game::ess_glue_social::on_tick(game);
}
