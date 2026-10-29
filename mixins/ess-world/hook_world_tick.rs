// Runs in native_tick after the core's hook (same priority, later mod id): this area's per-tick glue.
fn ess_world_tick(game: &mut crate::game::Game<'_>) {
    crate::game::ess_glue_world::on_tick(game);
}
