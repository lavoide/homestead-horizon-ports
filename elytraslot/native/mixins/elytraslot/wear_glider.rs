// Elytra Slot: the trinket glider wears.
fn elytraslot_wear_glider(game: &mut Game<'_>, points: u32) -> core::ops::ControlFlow<()> {
    if crate::game::elytraslot::wear_glider(game, points) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
