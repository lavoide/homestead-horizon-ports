// The cooking pot's screen: background, flame and arrow.
fn fd_screen_draw(game: &Game<'_>, block: &str, data: &[i32], batch: &mut HudBatch, origin: [f32; 2]) {
    crate::game::farmersdelight::screen_draw(game, block, data, batch, origin);
}
