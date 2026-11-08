// isValidBonemealTarget for the crops.
fn fd_bm_valid(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, valid: &mut Option<bool>) {
    if valid.is_none() {
        *valid = crate::game::farmersdelight::bonemeal_valid(level, pos, state);
    }
}
