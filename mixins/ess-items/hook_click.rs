// The first check of every container click: Essentials' plugin containers refuse what their policy refuses.
fn ess_items_click(
    game: &mut crate::game::Game<'_>,
    host: &mut crate::game::lan::LanHost,
    conn: u32,
    p: &horizon_protocol::packets::play::ContainerClick,
) -> std::ops::ControlFlow<bool> {
    crate::game::ess_glue_items::on_click(game, host, conn, p)
}
