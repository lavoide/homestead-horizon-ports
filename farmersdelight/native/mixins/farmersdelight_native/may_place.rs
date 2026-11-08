// mayPlaceOn of the panicles, colonies and tomatoes.
fn fd_may_place(level: &GameLevel<'_>, state: BlockStateId, ground: BlockStateId, may: &mut Option<bool>) {
    if may.is_none() {
        *may = crate::game::farmersdelight::may_place_on(level, state, ground);
    }
}
