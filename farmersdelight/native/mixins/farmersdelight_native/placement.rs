// getStateForPlacement of the blocks whose template places them otherwise.
fn fd_placement(level: &GameLevel<'_>, state: BlockStateId, context: &PlacementContext, fluid: FluidState, answer: &mut Option<Option<BlockStateId>>) {
    if answer.is_none() {
        *answer = crate::game::farmersdelight::placement(level, state, context, fluid);
    }
}
