// Hub::set_mod_commands: Essentials' labels join the server mods' commands in every player's tree.
fn ess_social_commands(hub: &mut Hub, key: u64, commands: &serde_json::Value) -> std::ops::ControlFlow<()> {
    let tree = crate::lan::hub::ess_social_hub::with_state(|state| {
        let tree = state.json["tree"].clone();
        if tree.is_object() {
            state.json["orig"] = commands.clone();
        }
        tree
    });
    if !tree.is_object() {
        return std::ops::ControlFlow::Continue(());
    }
    let mut merged = if commands.is_object() { commands.clone() } else { serde_json::json!({"type": "root"}) };
    if !merged["children"].is_object() {
        merged["children"] = serde_json::json!({});
    }
    if let (Some(dst), Some(src)) = (merged["children"].as_object_mut(), tree["children"].as_object()) {
        for (k, v) in src {
            dst.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    let resend = hub.mod_commands.0 != merged;
    hub.mod_commands = (merged, Some(key));
    if resend {
        for i in 0..hub.guests.len() {
            if hub.guests[i].placed {
                hub.send_commands(i);
            }
        }
    }
    std::ops::ControlFlow::Break(())
}
