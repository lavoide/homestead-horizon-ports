// A worn totem is ready.
fn trinkets_totem_ready(game: &mut Game<'_>) -> core::ops::ControlFlow<()> {
    if crate::game::trinkets::totem_ready(game) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
