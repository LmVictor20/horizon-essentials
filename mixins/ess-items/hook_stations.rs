// Portable workstations (/anvil, /loom...) are out of the play state while the stations' block checks run.
fn ess_items_stations(host: &mut crate::game::lan::LanHost) {
    crate::game::ess_glue_items::hide_portable_stations(host);
}
