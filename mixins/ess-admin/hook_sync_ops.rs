// HEAD of dedicated Game::sync_ops: keep the shared operator/whitelist/ban lists for the admin glue.
fn ess_admin_lists(lists: &std::sync::Arc<std::sync::Mutex<crate::game::dedicated::Lists>>) {
    crate::game::dedicated::ess_admin_dedicated::remember(lists);
}
