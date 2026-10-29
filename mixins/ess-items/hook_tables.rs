// The next call after lan_stations_tick: the portable workstations come back.
fn ess_items_tables(host: &mut crate::game::lan::LanHost) {
    crate::game::ess_glue_items::restore_portable_stations(host);
}
