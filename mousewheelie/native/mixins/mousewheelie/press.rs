// MixinAbstractContainerScreen.onMouseClick: a left click with Alt, Ctrl or Space held.
fn mw_press(game: &mut Game<'_>, right: bool) -> core::ops::ControlFlow<()> {
    crate::game::mousewheelie::press(game, right)
}
