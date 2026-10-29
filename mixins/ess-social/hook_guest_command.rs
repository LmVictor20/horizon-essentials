// Game::guest_command: vanilla runs /say, /me and /msg (tell, w) itself before the command hook; when Essentials
// has the label, the line goes to Essentials instead (signed arguments were already checked by the caller).
fn ess_social_guest_command(game: &mut Game<'_>, host: &mut LanHost, conn: u32, command: &str) -> std::ops::ControlFlow<()> {
    let uuid = host.hub.guests.iter().find(|g| g.conn == conn).map(|g| g.uuid);
    if crate::game::ess_glue_social::claim_message_command(game, uuid, command) {
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
