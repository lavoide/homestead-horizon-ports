// Rich soil farmland buried: rich soil again.
fn fd_tick_block(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    if !*handled && crate::game::farmersdelight::tick_block(level, ticks, pos, state) {
        *handled = true;
    }
}
