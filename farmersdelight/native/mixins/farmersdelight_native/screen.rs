// The cooking pot's screen layout.
fn fd_screen(block: &str, screen: &mut Option<NativeScreen>) {
    if screen.is_none()
        && let Some(s) = crate::game::farmersdelight::screen(block)
    {
        *screen = Some(s);
    }
}
