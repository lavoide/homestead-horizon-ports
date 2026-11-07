// A click on a chimney: blocked toggled, or soot scraped; anything else goes on.
fn sootychimneys_use(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3], taken: &mut bool) -> core::ops::ControlFlow<()> {
    if crate::game::sootychimneys::use_chimney(game, at, side) {
        *taken = true;
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
