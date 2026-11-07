// Replaces the body: the attribute times the block's factor.
fn speed(world: &mut MotionWorld<'_, '_, '_>) -> core::ops::ControlFlow<Result<f32>> {
    core::ops::ControlFlow::Break(crate::motion::blockrunner::speed(world))
}
