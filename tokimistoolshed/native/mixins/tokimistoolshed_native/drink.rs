// Copper milk drunk: the bucket back with one more use of wear.
fn tokimistoolshed_drink(game: &mut Game<'_>, info: &ItemInfo, consumable: &ItemConsumable, stack: &ItemStack) -> core::ops::ControlFlow<()> {
    crate::game::tokimistoolshed::finish_drinking(game, info, consumable, stack)
}
