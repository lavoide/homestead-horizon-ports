// A hammock or a sleeping bag used on a block: taken when Comforts handles it.
fn comforts_use_on(
    game: &mut Game<'_>,
    item: &ItemStack,
    at: [i32; 3],
    side: [i32; 3],
    taken: &mut bool,
) -> core::ops::ControlFlow<()> {
    if crate::game::comforts::use_on(game, item, at, side) {
        *taken = true;
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
