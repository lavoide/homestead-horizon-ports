// A click on the re-roll button is the button's.
fn em_click(game: &mut Game<'_>, right: bool) -> core::ops::ControlFlow<bool> {
    match crate::game::stations::easymagic::click(game, right) {
        Some(used) => core::ops::ControlFlow::Break(used),
        None => core::ops::ControlFlow::Continue(()),
    }
}
