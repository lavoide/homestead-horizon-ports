// The Trinkets key, in the window.
fn trinkets_key(game: &mut Game<'_>, key: &str, pressed: bool) {
    crate::game::trinkets::key(game, key, pressed);
}
