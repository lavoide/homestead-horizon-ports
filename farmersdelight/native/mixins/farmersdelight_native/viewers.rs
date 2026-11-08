// Cabinets open while someone looks inside.
fn fd_viewers(game: &mut Game<'_>, pos: [i32; 3], viewers: u32, previous: u32) {
    crate::game::farmersdelight::viewers(game, pos, viewers, previous);
}
