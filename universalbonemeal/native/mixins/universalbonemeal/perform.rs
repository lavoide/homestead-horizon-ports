// performBonemeal for a block Universal Bone Meal claimed.
fn grow(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    crate::game::universalbonemeal::perform(level, ticks, pos, state);
}
