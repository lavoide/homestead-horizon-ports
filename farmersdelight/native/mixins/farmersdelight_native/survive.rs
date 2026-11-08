// canSurvive of the vines, panicles and colonies.
fn fd_survive(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    if survives.is_none() {
        *survives = crate::game::farmersdelight::can_survive(level, pos, state);
    }
}
