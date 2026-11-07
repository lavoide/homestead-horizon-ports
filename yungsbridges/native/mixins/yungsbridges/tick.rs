// Bridges for the queued chunks that are ready.
fn yungsbridges_tick(game: &mut Game<'_>) {
    crate::game::yungsbridges::tick(game);
}
