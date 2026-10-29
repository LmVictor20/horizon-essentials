// Before a guest uses their held item on a block: the stack as it was (unlimited items).
fn ess_items_use(host: &crate::game::lan::LanHost, gi: usize) {
    crate::game::ess_glue_items::before_use(host, gi);
}
