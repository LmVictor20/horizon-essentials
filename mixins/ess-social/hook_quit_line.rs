// Hub::system_message: vanilla's "X left the game" is dropped when Essentials sends its own quit line.
fn ess_social_quit_line(hub: &Hub, content: &Tag) -> std::ops::ControlFlow<()> {
    let left = content.get("translate").is_some_and(|t| matches!(t, Tag::String(s) if s.to_string_lossy() == "multiplayer.player.left"));
    if left && hub.__horizon_mixins_horizon_essentials_social_ess().0["quit"] == true {
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
