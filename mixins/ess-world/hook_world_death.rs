// First in guest_forget_digging, which guest_die calls for a dying guest before anything drops: keepinv /
// keepxp and the curse policies (the handler does nothing for a living guest).
fn ess_world_death(host: &mut LanHost, conn: u32) {
    crate::game::ess_glue_world::on_guest_die(host, conn);
}
