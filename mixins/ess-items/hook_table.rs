// A portable crafting table (/workbench) sits at VEHICLE_KEY_Y: always valid.
fn ess_items_table(pos: [i32; 3]) -> std::ops::ControlFlow<bool> {
    if crate::game::ess_glue_items::table_valid(false, pos) {
        return std::ops::ControlFlow::Break(true);
    }
    std::ops::ControlFlow::Continue(())
}
