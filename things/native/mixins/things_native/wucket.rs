// The Bater Wucket used as a water bucket, then given back full.
fn things_wucket(game: &mut Game<'_>, stack: &ItemStack, allow: bool) -> core::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    crate::game::things::use_wucket(game, stack, allow)
}
