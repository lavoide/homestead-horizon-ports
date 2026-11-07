// A right click on a grave is the grave's.
fn ug_open(game: &mut Game<'_>) -> core::ops::ControlFlow<Result<Option<Hand>>> {
    if crate::game::universalgraves::open(game) {
        core::ops::ControlFlow::Break(Ok(Some(Hand::Main)))
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
