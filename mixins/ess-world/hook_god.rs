// HEAD of PlayerMotion::hurt_caused_by, where every hurt of a player passes: god mode (`essentials:god` on the
// body) and Protect's damage rules (`essentials:nodmg_<damage type>`) cancel it, as EssentialsX cancels the
// EntityDamageEvent.
fn ess_god(
    motion: &mut PlayerMotion,
    resources: &HealthResources,
    source: &DamageTypeId,
) -> core::ops::ControlFlow<Result<BlockOutcome>> {
    let name = resources
        .damage
        .definition(*source)
        .map(|d| d.name.trim_start_matches("minecraft:").to_string())
        .unwrap_or_default();
    let wanted = format!("essentials:nodmg_{name}");
    let mut god = false;
    let mut blocked = false;
    for (_, _, inst) in motion.attributes.instances() {
        if inst.modifier("essentials:god").is_some() {
            god = true;
        }
        if inst.modifier(&wanted).is_some() {
            blocked = true;
        }
    }
    if god {
        motion.fire_ticks = 0;
    }
    if god || blocked {
        return core::ops::ControlFlow::Break(Ok(BlockOutcome::default()));
    }
    core::ops::ControlFlow::Continue(())
}
