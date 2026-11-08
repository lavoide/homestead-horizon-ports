// isBonemealSuccess: always, for the crops.
fn fd_bm_success(level: &mut GameLevel<'_>, pos: BlockPos, state: BlockStateId, success: &mut bool) {
    if crate::game::farmersdelight::bonemeal_valid(level, pos, state).is_some() {
        *success = true;
    }
}
