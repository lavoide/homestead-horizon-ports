// BonemealCallback: claim the blocks Universal Bone Meal grows (isValidBonemealTarget).
fn claim(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, valid: &mut Option<bool>) {
    if valid.is_none() && crate::game::universalbonemeal::is_valid(level, pos, state) {
        *valid = Some(true);
    }
}
