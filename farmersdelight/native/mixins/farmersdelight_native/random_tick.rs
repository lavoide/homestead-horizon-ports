// The crops' and soils' random ticks.
fn fd_random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    if !*handled && crate::game::farmersdelight::random_tick(level, ticks, pos, state) {
        *handled = true;
    }
}
