// MinecraftClientMixin.onItemUse: no block aimed at, so use the bridging target as the hit.
fn bridge_local(game: &mut Game<'_>, block_hit: Option<&Hit>) -> core::ops::ControlFlow<Result<bool>> {
    if block_hit.is_none()
        && let Some(hit) = crate::game::bridgingmod::target(game)
    {
        return core::ops::ControlFlow::Break(game.use_hand(Some(&hit)));
    }
    core::ops::ControlFlow::Continue(())
}
