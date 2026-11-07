// A click in the trinket menu.
fn trinkets_clicked(game: &mut Game<'_>, uuid: u128, window: i32, slot: i32) -> core::ops::ControlFlow<()> {
    if crate::game::trinkets::clicked(game, uuid, window, slot) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
