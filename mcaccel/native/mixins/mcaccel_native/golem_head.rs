// The mod's pumpkins are golem heads too.
fn mcaccel_golem_head(game: &Game<'_>, at: [i32; 3]) -> core::ops::ControlFlow<bool> {
    if crate::game::mcaccel::is_head(game, at) {
        core::ops::ControlFlow::Break(true)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
