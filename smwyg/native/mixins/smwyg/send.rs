// ChatScreenMixin.sendMessage: a message holding the shared item's name goes as `smwyg:share`.
fn share_send(game: &mut Game<'_>, message: &str) -> core::ops::ControlFlow<()> {
    if crate::game::smwyg::send(game, message) {
        return core::ops::ControlFlow::Break(());
    }
    core::ops::ControlFlow::Continue(())
}
