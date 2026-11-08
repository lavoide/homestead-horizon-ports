// The items on the stove, skillet and board, drawn.
fn fd_submit(game: &Game<'_>, pos: [i32; 3], state: BlockStateId, entity: &ModBlockEntity, draw: &mut BlockEntityDraw) {
    crate::game::farmersdelight::submit(game, pos, state, entity, draw);
}
