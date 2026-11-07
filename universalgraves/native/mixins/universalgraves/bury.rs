// The death drops go into a grave when one can be made.
fn ug_bury(game: &mut Game<'_>) -> core::ops::ControlFlow<()> {
    if crate::game::universalgraves::bury(game) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
