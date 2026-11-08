// MixinAbstractContainerScreen.mouseWheelie_onMouseScroll: the wheel over a slot.
fn mw_scroll(game: &mut Game<'_>, delta: f32, slot: Option<usize>) -> core::ops::ControlFlow<bool> {
    crate::game::mousewheelie::scroll(game, delta, slot)
}
