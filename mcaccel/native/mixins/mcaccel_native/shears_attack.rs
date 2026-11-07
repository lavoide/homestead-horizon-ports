// Hitting a pumpkin of the cycle with shears: the previous face.
fn mcaccel_shears_attack(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3]) {
    crate::game::mcaccel::shears_attack(game, at, side);
}
