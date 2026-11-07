// Elytra Slot: a glider in a trinket slot glides.
fn elytraslot_glider(value: bool, game: &Game<'_>) -> bool {
    crate::game::elytraslot::glider_worn(game, value)
}
