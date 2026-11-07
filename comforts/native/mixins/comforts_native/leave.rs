// A player got up: a bag they put down to sleep in is taken up.
fn comforts_leave(game: &Game<'_>, who: u128, head: [i32; 3]) {
    crate::game::comforts::left_bed(game, who, head);
}
