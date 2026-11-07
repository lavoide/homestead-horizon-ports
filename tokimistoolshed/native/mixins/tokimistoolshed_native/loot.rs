// Clippers roll block loot as shears.
fn tokimistoolshed_loot(game: &mut Game<'_>, at: [i32; 3], state: BlockStateId, held: Option<&ItemStack>, entity: &Option<horizon_world::nbt::Tag>) -> core::ops::ControlFlow<Vec<ItemStack>> {
    crate::game::tokimistoolshed::loot(game, at, state, held, entity)
}
