// SortKeyBinding: the middle button sorts the part of the screen under the cursor.
fn mw_sort(game: &mut Game<'_>, button: MouseButton) -> core::ops::ControlFlow<bool> {
    crate::game::mousewheelie::sort_press(game, button)
}
