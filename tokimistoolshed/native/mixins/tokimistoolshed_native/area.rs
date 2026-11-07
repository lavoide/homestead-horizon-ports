// An area tool's or lumber axe's break: the block, then the rest.
fn tokimistoolshed_area(game: &mut Game<'_>, at: [i32; 3]) -> core::ops::ControlFlow<Result<()>> {
    crate::game::tokimistoolshed::break_block(game, at)
}
