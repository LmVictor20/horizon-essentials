// Runs first in break_block_as_player: the block_break listeners may keep the block (Break(Ok(()))).
fn ess_economy_break(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> ::core::ops::ControlFlow<Result<()>> {
    if crate::game::ess_glue_economy::on_break(game, pos) {
        return ::core::ops::ControlFlow::Break(Ok(()));
    }
    ::core::ops::ControlFlow::Continue(())
}
