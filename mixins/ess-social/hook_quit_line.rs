// Hub::system_message: vanilla's "X left the game" is dropped when Essentials sends its own quit line.
fn ess_social_quit_line(hub: &Hub, content: &Tag) -> std::ops::ControlFlow<()> {
    let _ = hub;
    let left = content.get("translate").is_some_and(|t| matches!(t, Tag::String(s) if s.to_string_lossy() == "multiplayer.player.left"));
    if left && crate::lan::hub::ess_social_hub::with_state(|state| state.json["quit"] == true) {
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
