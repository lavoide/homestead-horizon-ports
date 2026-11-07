// Breaking a grave spills it, or nothing happens when it isn't the player's yet.
fn ug_break(game: &mut Game<'_>, pos: [i32; 3]) -> core::ops::ControlFlow<Result<()>> {
    if crate::game::universalgraves::broken(game, pos) {
        core::ops::ControlFlow::Break(Ok(()))
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
