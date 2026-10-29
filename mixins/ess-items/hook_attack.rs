// A player attacks an entity: a `{player}` powertool on another player runs instead of the hit.
fn ess_items_attack(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, id: i32) -> std::ops::ControlFlow<()> {
    crate::game::ess_glue_items::on_attack(game, host, conn, id)
}
