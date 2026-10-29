// Before Hub::broadcast_chat: a player's chat (the plain chat type, not /say or /me) waits for Essentials, which
// decides on the next tick (pass it on signed, drop it, or send its own lines). The chain already advanced.
fn ess_social_chat(hub: &mut Hub, message: &ChatMessage, bound: &play::ChatTypeBound, line: &String) -> std::ops::ControlFlow<()> {
    let chat_type = hub.chat_type;
    let state = hub.__horizon_mixins_horizon_essentials_social_ess_mut();
    if state.0["on"] != true || state.0["bypass"] == true || bound.chat_type != chat_type {
        return std::ops::ControlFlow::Continue(());
    }
    state.1.push((message.clone(), bound.clone(), line.clone()));
    std::ops::ControlFlow::Break(())
}
