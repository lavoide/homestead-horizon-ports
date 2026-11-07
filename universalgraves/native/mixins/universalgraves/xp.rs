// The experience of the death just buried goes into its grave.
fn ug_xp(game: &mut Game<'_>, value: u32) -> core::ops::ControlFlow<()> {
    if crate::game::universalgraves::take_xp(game, value) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
