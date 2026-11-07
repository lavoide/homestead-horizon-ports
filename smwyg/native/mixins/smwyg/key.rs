// Ctrl + the chat key over an item in an inventory: share it (ShowMeWhatYouGotClient's tick).
fn share_key(game: &mut Game<'_>, key: KeyCode) -> core::ops::ControlFlow<Result<()>> {
    if crate::game::smwyg::share_key(game, key) {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    core::ops::ControlFlow::Continue(())
}
