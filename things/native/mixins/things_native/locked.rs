// A locked container keeps the host out too.
fn things_locked(game: &mut Game<'_>, pos: [i32; 3]) -> core::ops::ControlFlow<()> {
    crate::game::things::host_locked(game, pos)
}
