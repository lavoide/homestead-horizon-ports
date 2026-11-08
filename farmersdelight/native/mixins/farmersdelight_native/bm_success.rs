// isBonemealSuccess: always for the crops; the cinnamon sapling's 45%.
fn fd_bm_success(level: &mut GameLevel<'_>, pos: BlockPos, state: BlockStateId, success: &mut bool) {
    if let Some(s) = crate::game::farmersdelight::bonemeal_success(level, pos, state) {
        *success = s;
    }
}
