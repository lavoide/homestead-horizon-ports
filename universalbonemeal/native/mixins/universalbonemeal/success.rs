// isBonemealSuccess for a block Universal Bone Meal claimed.
fn roll(level: &mut GameLevel<'_>, pos: BlockPos, state: BlockStateId, success: &mut bool) {
    if let Some(yes) = crate::game::universalbonemeal::is_success(level, pos, state) {
        *success = yes;
    }
}
