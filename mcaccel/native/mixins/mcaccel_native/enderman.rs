// A player wearing one of the mod's carved faces isn't staring.
fn mcaccel_enderman(world: &MobWorld<'_>) -> core::ops::ControlFlow<bool> {
    let head = world.player.as_ref().and_then(|p| p.head.as_deref());
    if head.is_some_and(|h| h.starts_with("mcaccel:carved_pumpkin_")) {
        core::ops::ControlFlow::Break(false)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
