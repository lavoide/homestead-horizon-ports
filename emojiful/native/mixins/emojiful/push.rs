// EmojiFontRenderer: a chat line shows its `:name:` emoji codes as the emoji.
fn emojiful_push(chat: &mut Chat, font: &crate::font::Font, message: &Message, focused: bool) -> core::ops::ControlFlow<()> {
    match crate::chat::emojiful::convert(message) {
        Some(m) => {
            chat.push_message(font, m, focused);
            core::ops::ControlFlow::Break(())
        }
        None => core::ops::ControlFlow::Continue(()),
    }
}
