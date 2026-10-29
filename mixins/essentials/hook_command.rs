// Runs first in native_command; setting *ran keeps vanilla from also handling the command.
fn ess_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    crate::game::ess_glue::on_command(game, command, ran);
}
