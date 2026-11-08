// EntityBlock.getTicker: the stove, pot, skillet and basket tick.
fn fd_ticks(block: &str, ticks: &mut bool) {
    if crate::game::farmersdelight::ticks(block) {
        *ticks = true;
    }
}
