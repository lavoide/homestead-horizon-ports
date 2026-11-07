// The trinkets drop where the player died (when no grave took them).
fn trinkets_death(game: &mut Game<'_>, at: [f64; 3]) {
    crate::game::trinkets::drop_on_death(game, at);
}
