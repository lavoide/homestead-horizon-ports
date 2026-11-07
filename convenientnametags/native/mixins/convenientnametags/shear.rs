// ShearsItemMixin.useOnEntity: sneaking with shears on a named mob takes the name off as a tag
// and damages the shears by 1; the mob's own interaction (shearing a sheep...) doesn't run.
fn shear_name(game: &mut Game<'_>, target: Option<InteractTarget>) -> core::ops::ControlFlow<Result<bool>> {
    let sneaking = game.player.as_ref().is_some_and(|p| p.shift_key_down);
    if !sneaking || !game.held().is_some_and(|h| &*h.item == "minecraft:shears") {
        return core::ops::ControlFlow::Continue(());
    }
    let index = match target {
        Some(t) => Game::named_entity(&game.mobs, t.id, |m| m.id),
        None => game.mob_hit().map(|(i, _)| i),
    };
    let Some(index) = index else { return core::ops::ControlFlow::Continue(()) };
    let m = &game.mobs[index];
    if !m.alive() {
        return core::ops::ControlFlow::Continue(());
    }
    let Some(name) = m.custom_name.clone() else {
        return core::ops::ControlFlow::Continue(());
    };
    let m = m.clone();
    crate::game::creatures::convenientnametags::drop_tag(game, &m, &name);
    game.mobs[index].custom_name = None;
    game.damage_held_tool();
    game.swing_arm();
    core::ops::ControlFlow::Break(Ok(true))
}
