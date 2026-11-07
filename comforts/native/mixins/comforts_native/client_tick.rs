// The sleeper's height, right after the client's level tick.
fn comforts_client_tick(game: &mut Game<'_>) {
    crate::game::comforts::client_tick(game);
}
