// MixinAbstractContainerScreen.onMouseDragged: modifiers held while dragging over slots.
fn mw_drag(game: &mut Game<'_>) -> core::ops::ControlFlow<()> {
    crate::game::mousewheelie::drag(game)
}
