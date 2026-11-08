// Seeds, rope and hoes on rich soil.
fn fd_use_on(game: &mut Game<'_>, item: &ItemStack, at: [i32; 3], side: [i32; 3], taken: &mut bool) -> core::ops::ControlFlow<()> {
    if !*taken && crate::game::farmersdelight::use_on(game, item, at, side) {
        *taken = true;
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
