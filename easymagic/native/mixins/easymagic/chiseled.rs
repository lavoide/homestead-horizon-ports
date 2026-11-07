// The power the count found, plus a third of the books in chiseled bookshelves facing the table.
fn em_chiseled(value: i32, game: &Game<'_>, pos: [i32; 3]) -> i32 {
    value + crate::game::stations::easymagic::chiseled_power(game, pos)
}
