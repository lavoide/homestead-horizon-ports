// A crafting table's click (game.rs, the block-interaction branch) sets `craft_size = 3` and
// `craft_table = Some(pos)`. On the server's turn for a player (single player's server thread,
// a LAN guest) lan_actions.rs then opens a crafting window at that position. The pad does the
// same with a position no block has; valid.rs keeps that window open.
fn open_pad(game: &mut Game<'_>) -> core::ops::ControlFlow<Result<bool>> {
    if !game.held().is_some_and(|s| &*s.item == "craftingpad:craftingpad") {
        return core::ops::ControlFlow::Continue(());
    }
    game.craft_size = 3;
    game.craft_table = Some([i32::MIN, i32::MIN, i32::MIN]);
    game.screen = Screen::Inventory;
    game.capture(false);
    game.right = false;
    game.swing_arm();
    core::ops::ControlFlow::Break(Ok(true))
}
