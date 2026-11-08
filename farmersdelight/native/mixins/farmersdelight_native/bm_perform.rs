// performBonemeal for the crops.
fn fd_bm_perform(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    crate::game::farmersdelight::bonemeal_perform(level, ticks, pos, state);
}
