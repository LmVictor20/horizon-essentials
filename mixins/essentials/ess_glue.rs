// horizon-essentials glue: the injected module `crate::game::ess_glue`, compiled only inside Horizon.

use horizon_client::text::Text;

/// A command typed by a player or the console, before vanilla's dispatcher. The client also reaches
/// `native_command` before sending a command to its server, so only a server-side game claims it.
pub(super) fn on_command(game: &mut super::Game<'_>, command: &str, ran: &mut bool) {
    if game.native_window() {
        return;
    }
    let (name, args) = command.split_once(' ').unwrap_or((command, ""));
    match name {
        "ping" | "eping" | "pong" | "epong" | "echo" | "eecho" => {
            game.system_message(Text::literal(super::ess_core::ping(args)));
            *ran = true;
        }
        _ => {}
    }
}
