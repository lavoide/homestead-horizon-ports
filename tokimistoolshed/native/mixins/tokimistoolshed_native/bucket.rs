// A copper bucket used as its vanilla twin, then given back with its wear.
fn tokimistoolshed_bucket(game: &mut Game<'_>, stack: &ItemStack, allow: bool) -> core::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    crate::game::tokimistoolshed::use_bucket(game, stack, allow)
}
