// Convenient Name Tags for Horizon: the name tag that comes off a mob.
//
// Injected as `crate::game::creatures::convenientnametags`.

use super::Game;
use horizon_client::{items::ItemStack, mobs::Mob};

/// `new ItemEntity(world, x, y, z, new ItemStack(NAME_TAG).setCustomName(name))`, spawned at the
/// mob's position.
pub(crate) fn drop_tag(game: &mut Game<'_>, m: &Mob, name: &str) {
    let mut tag = ItemStack::new("minecraft:name_tag", 1);
    tag.data_mut().custom_name = Some(horizon_world::nbt::Tag::String(name.into()));
    let pos = [m.position.x, m.position.y, m.position.z];
    game.spawn_item(tag, pos, super::super::dispenser::new_item_motion(), 0);
}
