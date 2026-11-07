// The table keeps its item and lapis when its menu closes.
fn em_keep(game: &mut Game<'_>, host: &mut LanHost, conn: u32, window: Option<i32>) {
    crate::game::lan_stations::easymagic::keep(game, host, conn, window);
}
