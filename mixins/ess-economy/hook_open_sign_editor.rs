// Runs first in lan_open_sign_editor: a right click on a sign with text is the interact event; an
// Essentials sign keeps its editor shut (Break).
fn ess_economy_open_sign_editor(
    game: &mut crate::game::Game<'_>,
    host: &mut crate::game::lan::LanHost,
    conn: u32,
    edit: &crate::game::sign_edit::SignEditor,
) -> ::core::ops::ControlFlow<()> {
    let pos = [edit.pos.0, edit.pos.1, edit.pos.2];
    crate::game::ess_glue_economy::on_open_sign_editor(game, host, conn, pos, edit.front)
}
