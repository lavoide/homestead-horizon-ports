// Shears on a pumpkin of the cycle: the next face (SUCCESS); anything else goes on to vanilla.
fn mcaccel_shears_use(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3]) -> core::ops::ControlFlow<Result<bool>> {
    match crate::game::mcaccel::shears_use(game, at, side) {
        Some(result) => core::ops::ControlFlow::Break(result.map(|()| true)),
        None => core::ops::ControlFlow::Continue(()),
    }
}
