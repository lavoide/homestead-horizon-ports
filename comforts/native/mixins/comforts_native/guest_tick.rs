// Each guest's sleep tick.
fn comforts_guest_tick(game: &mut Game<'_>, lan: &mut LanHost, index: usize) {
    crate::game::comforts::guest_tick(game, lan, index);
}
