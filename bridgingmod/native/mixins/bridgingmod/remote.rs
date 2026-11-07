// The same for a LAN client: `pick` answers `forced_hit` first, for this one use.
fn bridge_remote(game: &mut Game<'_>, hand: Hand) -> core::ops::ControlFlow<bool> {
    if game.forced_hit.is_none()
        && game.pick().is_none()
        && let Some(hit) = crate::game::bridgingmod::target(game)
    {
        game.forced_hit = Some(hit);
        let done = game.remote_use_item_on(hand);
        game.forced_hit = None;
        return core::ops::ControlFlow::Break(done);
    }
    core::ops::ControlFlow::Continue(())
}
