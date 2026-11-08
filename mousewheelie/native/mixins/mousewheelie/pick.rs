// MWClient.triggerPick: pick block while holding a tool or weapon picks the best one instead.
fn mw_pick(game: &mut Game<'_>, button: i32) -> core::ops::ControlFlow<Option<bool>> {
    crate::game::mousewheelie::pick(game, button)
}
