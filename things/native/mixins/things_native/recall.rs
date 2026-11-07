// The Recall Potion drunk: to the respawn point.
fn things_recall(game: &mut Game<'_>, stack: &ItemStack) {
    crate::game::things::recall(game, stack);
}
