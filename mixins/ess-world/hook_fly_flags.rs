// RETURN of Guest::ability_flags: a player Essentials lets fly (the `essentials:fly` modifier on their body) may
// fly outside creative, and is told when they fly.
fn ess_fly_flags(guest: &Guest, value: i8) -> i8 {
    let allowed = guest
        .motion
        .as_ref()
        .is_some_and(|m| m.attributes.instances().any(|(_, _, i)| i.modifier("essentials:fly").is_some()));
    if !allowed {
        return value;
    }
    value | 4 | if guest.flying { 2 } else { 0 }
}
