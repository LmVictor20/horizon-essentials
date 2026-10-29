// A portable crafting table (/workbench) is always valid.
fn ess_items_table(value: bool, pos: [i32; 3]) -> bool {
    crate::game::ess_glue_items::table_valid(value, pos)
}
