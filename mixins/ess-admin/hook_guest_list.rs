// Cancellable HEAD of Game::guest_command: a guest's `/list` is answered by Horizon's own ListPlayersCommand
// before any mod sees it. It is queued like any other guest command instead, so it reaches
// native_command (Essentials' /list), or vanilla's /list when Essentials leaves it.
fn ess_admin_guest_list(
    host: &mut crate::game::lan::LanHost,
    conn: u32,
    command: &str,
) -> core::ops::ControlFlow<()> {
    let line = command.trim_start_matches('/');
    if line.split(' ').next() != Some("list") || line == "list uuids" {
        return core::ops::ControlFlow::Continue(());
    }
    let Some(permission) = host.hub.guest(conn).map(|g| g.permission) else {
        return core::ops::ControlFlow::Continue(());
    };
    let integrated = host.hub.config.integrated;
    host.commands.push(crate::game::lan_actions::GuestCommand { conn, command: line.to_owned(), permission, integrated, quiet: false });
    core::ops::ControlFlow::Break(())
}
