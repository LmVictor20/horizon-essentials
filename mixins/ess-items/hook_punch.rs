// A punch (left click) by the guest whose turn it is: their powertool.
fn ess_items_punch(game: &mut crate::game::Game<'_>) {
    crate::game::ess_glue_items::on_punch(game);
}
