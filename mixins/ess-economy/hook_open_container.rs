// Runs first in lan_open_container: opening a chest, furnace... is that block's interact event; Break(false)
// keeps it shut ("nothing the guest can use opened").
fn ess_economy_open_container(
    game: &mut crate::game::Game<'_>,
    host: &mut crate::game::lan::LanHost,
    conn: u32,
    pos: (i32, i32, i32),
) -> ::core::ops::ControlFlow<bool> {
    crate::game::ess_glue_economy::on_open_container(game, host, conn, [pos.0, pos.1, pos.2])
}
