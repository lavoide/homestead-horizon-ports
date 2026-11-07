// The pad's window (open.rs's position) is always valid, as the Java mod's stillValid is.
fn pad_valid(pos: [i32; 3]) -> core::ops::ControlFlow<bool> {
    if pos == [i32::MIN, i32::MIN, i32::MIN] {
        core::ops::ControlFlow::Break(true)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
