// A bag or hammock is slept in by Comforts' rules; anything else is vanilla's bed.
fn comforts_sleep(game: &mut Game<'_>, at: [i32; 3]) -> core::ops::ControlFlow<()> {
    if crate::game::comforts::try_sleep(game, at) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
