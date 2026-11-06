// Before Hub::broadcast_chat: a player's chat (the plain chat type, not /say or /me) waits for Essentials, which
// decides on the next tick (pass it on signed, drop it, or send its own lines). The chain already advanced.
fn ess_social_chat(hub: &mut Hub, message: &ChatMessage, bound: &play::ChatTypeBound, line: &String) -> std::ops::ControlFlow<()> {
    let chat_type = hub.chat_type;
    let hold = crate::lan::hub::ess_social_hub::with_state(|state| {
        if state.json["on"] != true || state.json["bypass"] == true || bound.chat_type != chat_type {
            return false;
        }
        state.pending.push((message.clone(), bound.clone(), line.clone()));
        true
    });
    if hold {
        std::ops::ControlFlow::Break(())
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
