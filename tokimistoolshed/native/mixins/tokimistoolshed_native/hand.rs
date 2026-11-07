// The trowel, chisels and abacus in the main hand.
fn tokimistoolshed_hand(game: &mut Game<'_>, hit: Option<&Hit>) -> core::ops::ControlFlow<Result<bool>> {
    crate::game::tokimistoolshed::use_hand(game, hit)
}
