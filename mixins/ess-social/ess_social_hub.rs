use std::sync::Mutex;
use serde_json::Value;

pub type PendingChat = (
    crate::lan::hub::chat::ChatMessage,
    horizon_protocol::packets::play::ChatTypeBound,
    String,
);

#[derive(Default)]
pub struct SocialHubState {
    pub json: Value,
    pub pending: Vec<PendingChat>,
}

pub static STATE: Mutex<Option<SocialHubState>> = Mutex::new(None);

pub fn with_state<R>(f: impl FnOnce(&mut SocialHubState) -> R) -> R {
    let mut guard = STATE.lock().unwrap();
    let state = guard.get_or_insert_with(SocialHubState::default);
    f(state)
}
