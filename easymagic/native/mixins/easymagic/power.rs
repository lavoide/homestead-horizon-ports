// Easy Magic's count of the enchanting power around a table.
fn em_power(game: &Game<'_>, pos: [i32; 3]) -> core::ops::ControlFlow<i32> {
    core::ops::ControlFlow::Break(crate::game::stations::easymagic::power(game, pos))
}
