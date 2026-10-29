// HEAD of Hub::tick_level, before the inboxes are processed: a survival player Essentials lets fly starts and
// stops flying with their client (handlePlayerAbilities only accepts creative and spectator flight).
fn ess_fly_inbox(hub: &mut Hub) {
    for g in hub.guests.iter_mut() {
        if g.creative() || g.spectator() || g.inbox.iter().all(|p| !matches!(p, S::PlayerAbilities(_))) {
            continue;
        }
        let allowed = g
            .motion
            .as_ref()
            .is_some_and(|m| m.attributes.instances().any(|(_, _, inst)| inst.modifier("essentials:fly").is_some()));
        if !allowed {
            continue;
        }
        let mut flying = g.flying;
        g.inbox.retain(|p| match p {
            S::PlayerAbilities(a) => {
                flying = a.flags & 2 != 0;
                false
            }
            _ => true,
        });
        g.flying = flying;
    }
}
