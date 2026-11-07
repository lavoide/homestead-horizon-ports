// The wake-up time after a night (or a day) slept through.
fn comforts_wake_time(game: &Game<'_>, now: i64, wake: &mut i64) {
    crate::game::comforts::wake_time(game, now, wake);
}
