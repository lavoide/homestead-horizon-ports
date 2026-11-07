// NameTagItemMixin.useOnEntity: the same name passes (the tag stays); a different one drops the
// old name as a tag before the mob is renamed.
fn rename(game: &mut Game<'_>, index: usize) -> core::ops::ControlFlow<bool> {
    let Some(new) = game
        .held()
        .filter(|h| &*h.item == "minecraft:name_tag")
        .and_then(|h| h.custom_name())
    else {
        return core::ops::ControlFlow::Continue(());
    };
    let m = &game.mobs[index];
    match m.custom_name.clone() {
        Some(old) if old == new => core::ops::ControlFlow::Break(false),
        Some(old) if m.alive() => {
            let m = m.clone();
            crate::game::creatures::convenientnametags::drop_tag(game, &m, &old);
            core::ops::ControlFlow::Continue(())
        }
        _ => core::ops::ControlFlow::Continue(()),
    }
}
