// The stand-in chest keys of Essentials' plugin containers stay valid (no vehicle behind them).
fn ess_items_vehicle(id: i32) -> std::ops::ControlFlow<bool> {
    match crate::game::ess_glue_items::vehicle_valid(id) {
        Some(valid) => std::ops::ControlFlow::Break(valid),
        None => std::ops::ControlFlow::Continue(()),
    }
}
