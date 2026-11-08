// A tatami or full tatami mat placed: its pair.
fn fd_placed(game: &mut Game<'_>, pos: [i32; 3], state: BlockStateId) {
    crate::game::farmersdelight::placed(game, pos, state);
}
