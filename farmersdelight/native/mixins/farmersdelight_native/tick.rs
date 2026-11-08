// The machines' server ticks.
fn fd_tick(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    crate::game::farmersdelight::tick(game, pos, entity);
}
