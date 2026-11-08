// The drinks' own effects.
fn fd_consume(game: &mut Game<'_>, kind: &str) {
    crate::game::farmersdelight::consume_effect(game, kind);
}
