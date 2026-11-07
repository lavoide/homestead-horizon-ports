// Block Runner for Horizon: players run faster on paths and stone bricks, as Fuzs' Block Runner
// 8.0.4 does with Homestead's config (`minecraft:dirt_path` 1.35, `#minecraft:stone_bricks`
// 1.15; its own block tags are empty in the pack).
//
// Injected as `crate::motion::blockrunner`, a child of the motion module, so it can use
// MotionWorld's private fields and methods. The block is the one vanilla's
// `getBlockPosBelowThatAffectsMyMovement` picks (`MOVEMENT_SUPPORT_OFFSET`, 0.5 below the feet:
// a path is 15/16 tall, so feet stand inside it).

use super::{MotionWorld, movement_support};
use crate::Result;

/// The speed factor for standing on `block` (1.0: none).
pub fn factor(block: &str) -> f32 {
    match block {
        "minecraft:dirt_path" => 1.35,
        // `#minecraft:stone_bricks` in 26.3.
        "minecraft:stone_bricks"
        | "minecraft:mossy_stone_bricks"
        | "minecraft:cracked_stone_bricks"
        | "minecraft:chiseled_stone_bricks" => 1.15,
        _ => 1.0,
    }
}

/// The movement_speed attribute times the factor of the block below.
pub fn speed(world: &mut MotionWorld<'_, '_, '_>) -> Result<f32> {
    let base = world.player.attribute("minecraft:movement_speed")? as f32;
    let f = match world.support(movement_support::MOVEMENT_SUPPORT_OFFSET) {
        Ok(pos) => {
            let state = world.block(pos);
            world.assets.registry.state(state).map_or(1.0, |s| factor(&s.name))
        }
        Err(_) => 1.0,
    };
    Ok(base * f)
}

#[cfg(test)]
mod tests {
    use super::factor;

    #[test]
    fn factors() {
        assert_eq!(factor("minecraft:dirt_path"), 1.35);
        assert_eq!(factor("minecraft:mossy_stone_bricks"), 1.15);
        assert_eq!(factor("minecraft:stone_brick_stairs"), 1.0);
        assert_eq!(factor("minecraft:grass_block"), 1.0);
    }
}
