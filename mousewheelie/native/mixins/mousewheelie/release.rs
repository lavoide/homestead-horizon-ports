// The release of a press Mouse Wheelie took clicks nothing.
fn mw_release(game: &mut Game<'_>, right: bool) -> core::ops::ControlFlow<()> {
    crate::game::mousewheelie::release(game, right)
}
