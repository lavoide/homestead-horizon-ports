// EmojiSuggestionHelper: `:name` completions in the chat box.
fn emojiful_update(screen: &mut ChatScreen, m: &Measure, c: &Commands) -> core::ops::ControlFlow<()> {
    crate::chat::emojiful::update(screen, m, c)
}
