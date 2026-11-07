// A copper bucket milks a cow or mooshroom.
fn tokimistoolshed_milk(game: &mut Game<'_>, target: Option<InteractTarget>) -> core::ops::ControlFlow<Result<bool>> {
    crate::game::tokimistoolshed::milk(game, target)
}
