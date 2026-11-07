// Minecraft Accelerated 1.0.0 (pumpkinsAccelerated, mari, CC0-1.0): the shears cycle of carved
// faces, the mod's pumpkins as golem heads, and the worn face's blur. The blocks are the
// content mod mods/mcaccel.
//
// Injected as `crate::game::mcaccel`.
use super::*;

const FACES: [&str; 9] = ["x", "smile", "derp", "check", "creeper", "herobrine", "evil", "heart", "shout"];

/// The three cycles, vanilla's block first: CarvedPumpkinModifier.PUMPKIN_CHANGE_MAP goes forward
/// through them and LeftClickPumpkinEvent's reversed map backward.
fn cycles() -> [Vec<String>; 3] {
    let cycle = |first: &str, prefix: &str| {
        let mut c = vec![first.to_owned()];
        c.extend(FACES.iter().map(|f| format!("mcaccel:{prefix}_{f}")));
        c
    };
    [
        cycle("minecraft:carved_pumpkin", "carved_pumpkin"),
        cycle("minecraft:jack_o_lantern", "jack_o_lantern"),
        cycle("mcaccel:soul_jack_o_lantern_normal", "soul_jack_o_lantern"),
    ]
}

/// The block after (`forward`) or before `name` in its cycle.
pub(crate) fn step(name: &str, forward: bool) -> Option<String> {
    for (i, cycle) in cycles().iter().enumerate() {
        let Some(at) = cycle.iter().position(|n| n == name) else {
            continue;
        };
        if forward {
            // The soul cycle's forward map leaves its plain lantern out (shout goes back to x):
            // shears on a plain soul jack o'lantern do nothing (on Java they throw).
            if i == 2 {
                return (at != 0).then(|| cycle[at % 9 + 1].clone());
            }
            return Some(cycle[(at + 1) % cycle.len()].clone());
        }
        return Some(cycle[(at + cycle.len() - 1) % cycle.len()].clone());
    }
    None
}

fn name_at(game: &Game<'_>, p: [i32; 3]) -> std::sync::Arc<str> {
    game.res.facts().get(game.world.block(p[0], p[1], p[2])).name.clone()
}

fn holds_shears(game: &Game<'_>) -> bool {
    game.held().is_some_and(|s| &*s.item == "minecraft:shears")
}

/// The clicked side, or the player's opposite on the top and bottom (both handlers' blockDir).
fn facing(game: &Game<'_>, side: [i32; 3]) -> &'static str {
    let (dx, dz) = if side[1] != 0 {
        match ((game.yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4) {
            0 => (0, -1),
            1 => (1, 0),
            2 => (0, 1),
            _ => (-1, 0),
        }
    } else {
        (side[0], side[2])
    };
    match (dx, dz) {
        (0, -1) => "north",
        (0, 1) => "south",
        (-1, 0) => "west",
        _ => "east",
    }
}

fn state_of(game: &Game<'_>, name: &str, facing: &str) -> Option<horizon_world::registry::BlockStateId> {
    let reg = game.pack.assets.registry;
    let d = reg.default_state(name)?;
    let mut st = reg.state(d)?.clone();
    st.properties.insert("facing".into(), facing.into());
    reg.resolve(&st).ok()
}

/// Turns the pumpkin at `at` to `next`, as setBlockState does it: onPlace builds a golem.
fn turn(game: &mut Game<'_>, at: [i32; 3], next: &str, side: [i32; 3]) -> horizon_client::Result<bool> {
    let Some(state) = state_of(game, next, facing(game, side)) else {
        return Ok(false);
    };
    let center = at.map(|c| f64::from(c) + 0.5);
    game.audio.play("minecraft:block.pumpkin.carve", Some(center), 1.0, 1.0);
    game.set_golem_head(at, state)?;
    Ok(true)
}

/// CarvedPumpkinModifier.onUse with shears: the next face, one point of wear, the shear game
/// event and the used statistic. None when it isn't shears on a pumpkin of the cycle.
pub(crate) fn shears_use(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3]) -> Option<horizon_client::Result<()>> {
    if !holds_shears(game) {
        return None;
    }
    let next = step(&name_at(game, at), true)?;
    let shears = game.held().cloned()?;
    Some((|| {
        if !turn(game, at, &next, side)? {
            return Ok(());
        }
        game.damage_held(1);
        game.post_block_game_event("minecraft:shear", at, game.sculk_actor(), None);
        game.award_stat("used", "minecraft:shears", 1);
        game.trigger_block("minecraft:item_used_on_block", at, &shears);
        Ok(())
    })())
}

/// LeftClickPumpkinEvent: hitting one with shears turns it back a face. It answers PASS, so the
/// hit goes on (a creative player still breaks the block).
pub(crate) fn shears_attack(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3]) {
    if !holds_shears(game) {
        return;
    }
    let Some(previous) = step(&name_at(game, at), false) else {
        return;
    };
    if let Err(e) = turn(game, at, &previous, side) {
        log::warn!("[mcaccel] turning the pumpkin at {at:?} back: {e}");
    }
}

/// BlockInit.PUMPKIN_BLOCKS: every one of the mod's 28 blocks heads a golem.
pub(crate) fn is_head(game: &Game<'_>, at: [i32; 3]) -> bool {
    let name = name_at(game, at);
    name.starts_with("mcaccel:carved_pumpkin_")
        || name.starts_with("mcaccel:jack_o_lantern_")
        || name.starts_with("mcaccel:soul_jack_o_lantern_")
}

/// The worn carved face's blur (`mcaccel:gui/pumpkin_blur_<face>`).
pub(crate) fn blur(game: &Game<'_>) -> Option<String> {
    let worn = game.armor[0].as_ref()?;
    let face = worn.item.strip_prefix("mcaccel:carved_pumpkin_")?;
    FACES.contains(&face).then(|| format!("mcaccel:gui/pumpkin_blur_{face}"))
}

#[cfg(test)]
mod mcaccel_tests {
    use super::step;

    #[test]
    fn faces_cycle_both_ways() {
        assert_eq!(step("minecraft:carved_pumpkin", true).as_deref(), Some("mcaccel:carved_pumpkin_x"));
        assert_eq!(step("mcaccel:carved_pumpkin_shout", true).as_deref(), Some("minecraft:carved_pumpkin"));
        assert_eq!(step("minecraft:jack_o_lantern", false).as_deref(), Some("mcaccel:jack_o_lantern_shout"));
        assert_eq!(step("mcaccel:soul_jack_o_lantern_shout", true).as_deref(), Some("mcaccel:soul_jack_o_lantern_x"));
        assert_eq!(step("mcaccel:soul_jack_o_lantern_normal", true), None);
        assert_eq!(step("mcaccel:soul_jack_o_lantern_x", false).as_deref(), Some("mcaccel:soul_jack_o_lantern_normal"));
        assert_eq!(step("minecraft:pumpkin", true), None);
    }
}
