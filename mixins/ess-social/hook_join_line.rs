// Hub::broadcast_except: vanilla's "X joined the game" is dropped when Essentials sends its own join line
// (custom-join-message, silent join, hide-join-quit-messages-above).
fn ess_social_join_line(hub: &Hub, packet: &C) -> std::ops::ControlFlow<()> {
    let _ = hub;
    if let C::SystemChat(chat) = packet {
        let joined = chat.content.get("translate").is_some_and(|t| matches!(t, Tag::String(s) if s.to_string_lossy() == "multiplayer.player.joined"));
        if joined && crate::lan::hub::ess_social_hub::with_state(|state| state.json["join"] == true) {
            return std::ops::ControlFlow::Break(());
        }
    }
    std::ops::ControlFlow::Continue(())
}
