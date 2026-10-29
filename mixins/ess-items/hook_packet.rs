// Every packet a player sends, before the game handles it: plugin container clicks, powertools, unlimited items.
fn ess_items_packet(
    game: &mut crate::game::Game<'_>,
    host: &mut crate::game::lan::LanHost,
    conn: u32,
    packet: &horizon_protocol::packets::play::Serverbound,
) -> std::ops::ControlFlow<()> {
    crate::game::ess_glue_items::on_packet(game, host, conn, packet)
}
