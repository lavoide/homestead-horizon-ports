// Where a guest lies once asleep.
fn comforts_guest_height(value: [f64; 3], game: &Game<'_>) -> [f64; 3] {
    crate::game::comforts::sleep_top(game, value)
}
