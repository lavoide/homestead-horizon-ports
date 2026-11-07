// The Java mod's ItemFrame.interact HEAD: sneaking toggles `Invisible` and succeeds, before the
// item is put in or turned. The frame is picked as the original picks it.
fn toggle(game: &mut Game<'_>, target: Option<InteractTarget>) -> core::ops::ControlFlow<bool> {
    let sneaking = game.player.as_ref().is_some_and(|p| p.shift_key_down);
    if !sneaking {
        return core::ops::ControlFlow::Continue(());
    }
    let found = match target {
        Some(t) => Game::named_entity(&game.item_frames, t.id, |f| f.id).map(|i| (i, 0.0)),
        None => game.frame_hit(),
    };
    let Some((i, d)) = found else { return core::ops::ControlFlow::Continue(()) };
    if target.is_none() && !game.before_block(d) {
        return core::ops::ControlFlow::Continue(());
    }
    let f = &mut game.item_frames[i];
    f.invisible = !f.invisible;
    let p = f.pos;
    game.world.modified.insert((p[0] >> 4, p[2] >> 4));
    game.swing_arm();
    core::ops::ControlFlow::Break(true)
}
