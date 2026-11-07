// The trinkets' attribute modifiers, each player's in their own tick.
fn things_modifiers(game: &mut Game<'_>) {
    crate::game::things::trinket_modifiers(game);
}
