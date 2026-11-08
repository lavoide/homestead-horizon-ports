// The machines' item slots.
fn fd_container(block: &str, container: &mut Option<ModContainer>) {
    if container.is_none()
        && let Some(c) = crate::game::farmersdelight::container(block)
    {
        *container = Some(c);
    }
}
