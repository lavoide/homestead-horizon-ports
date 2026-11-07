// Menu button 4 on an enchanting table is the re-roll.
fn em_reroll(
    game: &mut Game<'_>,
    host: &mut LanHost,
    conn: u32,
    window: i32,
    id: i32,
) -> core::ops::ControlFlow<()> {
    if crate::game::lan_stations::easymagic::reroll(game, host, conn, window, id) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
