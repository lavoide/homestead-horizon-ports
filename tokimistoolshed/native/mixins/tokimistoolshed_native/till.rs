// A scythe tills the 3x3.
fn tokimistoolshed_till(game: &mut Game<'_>, stack: &ItemStack, at: [i32; 3], side: [i32; 3]) -> core::ops::ControlFlow<Result<bool>> {
    crate::game::tokimistoolshed::till(game, stack, at, side)
}
