// TrinketItem.use: a right click wears the trinket.
fn trinkets_use(game: &mut Game<'_>, stack: &ItemStack, claimed: &mut bool) -> core::ops::ControlFlow<()> {
    if !*claimed && crate::game::trinkets::use_item(game, stack) {
        *claimed = true;
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
