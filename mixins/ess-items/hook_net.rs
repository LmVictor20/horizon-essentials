// Item components Horizon keeps but does not send (lore, the head profile, unbreakable, custom model data).
fn ess_items_net(hub: &mut crate::lan::hub::Hub, s: &Option<crate::items::ItemStack>) -> std::ops::ControlFlow<Option<horizon_protocol::item::Stack>> {
    ess_items_net::stack(hub, s)
}
