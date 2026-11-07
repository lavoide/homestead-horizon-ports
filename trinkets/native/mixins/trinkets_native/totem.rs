// A worn totem saves the player.
fn trinkets_totem(game: &mut Game<'_>) -> core::ops::ControlFlow<Option<Body>> {
    match crate::game::trinkets::apply_worn_totem(game) {
        Some(body) => core::ops::ControlFlow::Break(body),
        None => core::ops::ControlFlow::Continue(()),
    }
}
