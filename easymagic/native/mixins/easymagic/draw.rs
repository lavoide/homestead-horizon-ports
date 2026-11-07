// The station as always, then the re-roll button (client.rs re-enters draw_station).
fn em_draw(game: &mut Game<'_>, batch: &mut HudBatch) -> core::ops::ControlFlow<bool> {
    match crate::game::stations::easymagic::draw(game, batch) {
        Some(shown) => core::ops::ControlFlow::Break(shown),
        None => core::ops::ControlFlow::Continue(()),
    }
}
