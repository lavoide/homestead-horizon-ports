// The blocks' use, before the block's own and the item's.
fn fd_click(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3], taken: &mut bool) -> core::ops::ControlFlow<()> {
    if !*taken && crate::game::farmersdelight::right_click(game, at, side) {
        *taken = true;
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
