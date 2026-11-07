// Bridging Mod 2.5.1 (CG360, MIT): the bridging target (PathTraversalHandler.getClosestAssistTarget
// and MinecraftClientMixin's slab assist), with Homestead's config: assist on, not only when
// crouched, both axes, non-solid blocks replaced, slab assist on, torches skipped.
use super::*;

/// Direction.values() order: down, up, north, south, west, east.
const DIRECTIONS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// GameSupport.isStackPlaceable: a block item, not a torch.
fn placeable(game: &Game<'_>, stack: Option<&ItemStack>) -> bool {
    let Some(stack) = stack else {
        return false;
    };
    let Some(block) = game.items().get(&stack.item).and_then(|i| i.block) else {
        return false;
    };
    // skipTorchBridging: TorchBlock (torch, soul torch, copper torch and their wall forms).
    let name = &game.res.facts().get(block).name;
    !(name.ends_with("torch") && !name.ends_with("redstone_torch"))
}

/// The bridging hit, when nothing is aimed at and a hand holds a block: a click on the face of the
/// block the target is built against.
pub(crate) fn target(game: &Game<'_>) -> Option<Hit> {
    if game.spectator() || game.player.is_none() {
        return None;
    }
    if !placeable(game, game.held()) && !placeable(game, game.offhand.as_ref()) {
        return None;
    }
    let (origin, dir) = game.aim()?;
    let reach = game.block_reach;
    let facts = game.res.facts();
    let state = |p: [i32; 3]| facts.get(game.world.block(p[0], p[1], p[2]));
    let p = game.player.as_ref()?;
    let w = f64::from(p.dimensions.width) / 2.0;
    let (lo, hi) = (
        [p.position.x - w, p.position.y, p.position.z - w],
        [p.position.x + w, p.position.y + f64::from(p.dimensions.height), p.position.z + w],
    );
    // getValidAssistSides: the faces the view looks into (dot >= 0.1), as the side to build off.
    let sides: Vec<[i32; 3]> = DIRECTIONS
        .iter()
        .filter(|d| dir[0] * d[0] as f64 + dir[1] * d[1] as f64 + dir[2] * d[2] as f64 >= 0.1)
        .map(|d| [-d[0], -d[1], -d[2]])
        .collect();
    for pos in path(origin, dir, reach) {
        // isBridgingPlacementAllowedAt (enableNonSolidReplace: canBeReplaced).
        if !state(pos).replaceable {
            continue;
        }
        // Not inside the player.
        // (1e-7: a box edge on a block boundary, -3.7 - 0.3, isn't inside the next block.)
        let inside = (0..3).all(|i| lo[i] < f64::from(pos[i]) + 1.0 - 1e-7 && hi[i] > f64::from(pos[i]) + 1e-7);
        if inside {
            continue;
        }
        for side in &sides {
            // canSideBeBuiltOffOf: not air, not a liquid, not replaceable.
            let off = [pos[0] + side[0], pos[1] + side[1], pos[2] + side[2]];
            let s = state(off);
            let air = s.name.ends_with(":air");
            let liquid = s.classes.iter().any(|c| &**c == "LiquidBlock");
            if air || liquid || s.replaceable {
                continue;
            }
            // Click the face of `off` that looks at `pos`.
            let face = [-side[0], -side[1], -side[2]];
            let mut point = [
                f64::from(pos[0]) + 0.5 + 0.5 * f64::from(side[0]),
                f64::from(pos[1]) + 0.5 + 0.5 * f64::from(side[1]),
                f64::from(pos[2]) + 0.5 + 0.5 * f64::from(side[2]),
            ];
            if side[1] == 0 {
                // Horizontal slab assist: standing 0.1775..0.51 into a block (on a bottom slab)
                // fills the lower half, else the upper.
                let frac = p.position.y - p.position.y.floor();
                point[1] = f64::from(pos[1]) + if frac > 0.1775 && frac < 0.51 { 0.1 } else { 0.9 };
            }
            return Some(Hit { pos: off, face, point });
        }
    }
    None
}

/// The blocks along the view from 1 block out to `reach` (getViewBlockPath), in order.
fn path(origin: [f64; 3], dir: [f64; 3], reach: f64) -> Vec<[i32; 3]> {
    let start: [f64; 3] = std::array::from_fn(|i| origin[i] + dir[i]);
    let mut pos = start.map(|v| v.floor() as i32);
    let step = dir.map(|d| if d > 0.0 { 1 } else { -1 });
    let delta = dir.map(|d| if d == 0.0 { f64::INFINITY } else { (1.0 / d).abs() });
    let mut t_max = [0.0; 3];
    for i in 0..3 {
        let boundary = if dir[i] > 0.0 { f64::from(pos[i]) + 1.0 } else { f64::from(pos[i]) };
        t_max[i] = if dir[i] == 0.0 { f64::INFINITY } else { (boundary - start[i]) / dir[i] };
    }
    let mut out = vec![pos];
    let length = (reach - 1.0).max(0.0);
    loop {
        let axis = (0..3).min_by(|&a, &b| t_max[a].total_cmp(&t_max[b])).unwrap_or(0);
        if t_max[axis] > length {
            return out;
        }
        pos[axis] += step[axis];
        t_max[axis] += delta[axis];
        out.push(pos);
    }
}
