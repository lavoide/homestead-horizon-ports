// Universal Bone Meal 8.0.1 (Fuzs, MPL-2.0) for Horizon: the bone meal behaviours its
// UniversalBoneMeal.registerBonemealBehaviors registers, with Homestead's config (all on). Each
// one answers vanilla's three BonemealableBlock questions for its blocks. Ported from the
// decompiled `fuzs.universalbonemeal.world.level.block.behavior` classes.
use super::*;
use horizon_core::random::LegacyRandom;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// Cactus and sugar cane: SimpleGrowingPlantBehavior.
    Tall,
    /// VineBehavior: grows down from the lowest vine.
    Vine,
    NetherWart,
    FruitStem,
    /// SimpleSpreadBehavior(width, most successes).
    Spread(i32, i32),
    Coral,
    ChorusFlower,
    ChorusPlant,
    Mycelium,
    Dirt,
    Podzol,
    SporeBlossom,
}

const SMALL_FLOWERS: &[&str] = &[
    "minecraft:dandelion",
    "minecraft:poppy",
    "minecraft:blue_orchid",
    "minecraft:allium",
    "minecraft:azure_bluet",
    "minecraft:red_tulip",
    "minecraft:orange_tulip",
    "minecraft:white_tulip",
    "minecraft:pink_tulip",
    "minecraft:oxeye_daisy",
    "minecraft:cornflower",
    "minecraft:lily_of_the_valley",
];
// The coral tags in their data order (#minecraft:coral_blocks, #coral_plants, #wall_corals).
const CORAL_TYPES: &[&str] = &["tube", "brain", "bubble", "fire", "horn"];

fn kind(b: &Behaviours, state: BlockStateId) -> Option<Kind> {
    let name = b.table.name(state);
    Some(match name {
        "minecraft:cactus" | "minecraft:sugar_cane" => Kind::Tall,
        "minecraft:vine" => Kind::Vine,
        "minecraft:nether_wart" => Kind::NetherWart,
        "minecraft:melon_stem" | "minecraft:pumpkin_stem" => Kind::FruitStem,
        "minecraft:lily_pad" => Kind::Spread(4, 3),
        "minecraft:dead_bush" => Kind::Spread(4, 2),
        "minecraft:chorus_flower" => Kind::ChorusFlower,
        "minecraft:chorus_plant" => Kind::ChorusPlant,
        "minecraft:mycelium" => Kind::Mycelium,
        "minecraft:dirt" | "minecraft:coarse_dirt" | "minecraft:dirt_path" => Kind::Dirt,
        "minecraft:podzol" => Kind::Podzol,
        "minecraft:spore_blossom" => Kind::SporeBlossom,
        n if SMALL_FLOWERS.contains(&n) => Kind::Spread(3, 1),
        _ if in_tag(b, state, "minecraft:corals") => Kind::Coral,
        _ => return None,
    })
}

fn in_tag(b: &Behaviours, state: BlockStateId, tag: &str) -> bool {
    b.tag(tag).is_some_and(|set| set.contains(b.block(state)))
}
fn is(b: &Behaviours, state: BlockStateId, name: &str) -> bool {
    b.table.name(state) == name
}
fn default(b: &Behaviours, name: &str) -> Option<BlockStateId> {
    b.table.block_by_name(name).map(|id| b.table.default_state(id))
}
fn off(pos: BlockPos, dx: i32, dy: i32, dz: i32) -> BlockPos {
    BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz)
}
const HORIZONTAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

fn next_int(level: &mut GameLevel<'_>, bound: i32) -> i32 {
    level.random().next_int_bound(bound).unwrap_or(0)
}

/// `GrowingPlantBehavior.getTopConnectedBlock`: the last block of `state`'s kind going `dy`.
fn top_connected(level: &GameLevel<'_>, pos: BlockPos, block: &str, dy: i32) -> BlockPos {
    let b = level.behaviours;
    let mut p = pos;
    loop {
        let next = off(p, 0, dy, 0);
        if next.y < level.min_y() || next.y >= level.max_y() || !is(b, level.block_state(next), block) {
            return p;
        }
        p = next;
    }
}

/// `SimpleGrowingPlantBehavior.getMaxHeightAtPosition`: 12 + seedSlimeChunk(x, z, 0, 987234911)
/// .nextInt(5).
fn max_tall_height(x: i32, z: i32) -> i32 {
    let seed = (x.wrapping_mul(x).wrapping_mul(4_987_142) as i64)
        .wrapping_add(x.wrapping_mul(5_947_611) as i64)
        .wrapping_add((z.wrapping_mul(z) as i64).wrapping_mul(4_392_871))
        .wrapping_add(z.wrapping_mul(389_711) as i64)
        ^ 987_234_911;
    12 + LegacyRandom::new(seed).next_int_bound(5).unwrap_or(0)
}

/// `ChorusPlantBehavior.getTopConnectedBlock`: the chorus flowers reached by walking the plant's
/// connections (not down, not back), at most 128 deep.
fn chorus_flowers(level: &GameLevel<'_>, start: BlockPos) -> Vec<BlockPos> {
    let b = level.behaviours;
    let p = b.props;
    let mut out = Vec::new();
    fn walk(level: &GameLevel<'_>, pos: BlockPos, from: Option<(i32, i32, i32)>, depth: i32, out: &mut Vec<BlockPos>) {
        let b = level.behaviours;
        let p = b.props;
        let state = level.block_state(pos);
        if depth > 0 && is(b, state, "minecraft:chorus_plant") {
            for (key, d) in [
                (p.north, (0, 0, -1)),
                (p.east, (1, 0, 0)),
                (p.south, (0, 0, 1)),
                (p.west, (-1, 0, 0)),
                (p.up, (0, 1, 0)),
            ] {
                if Some(d) != from && b.table.bool(state, key) {
                    walk(level, off(pos, d.0, d.1, d.2), Some((-d.0, -d.1, -d.2)), depth - 1, out);
                }
            }
        } else if is(b, state, "minecraft:chorus_flower") && !out.contains(&pos) {
            out.push(pos);
        }
    }
    let _ = p;
    walk(level, start, Some((0, -1, 0)), 128, &mut out);
    out
}

fn chorus_flower_valid(level: &GameLevel<'_>, state: BlockStateId) -> bool {
    let b = level.behaviours;
    is(b, state, "minecraft:chorus_flower") && b.table.int(state, b.props.age) < 5
}

/// `canSustainPlant` for a stem's fruit: the ground the Java mod accepts.
fn fruit_ground(b: &Behaviours, state: BlockStateId) -> bool {
    [
        "minecraft:farmland",
        "minecraft:dirt",
        "minecraft:coarse_dirt",
        "minecraft:podzol",
        "minecraft:grass_block",
    ]
    .contains(&b.table.name(state))
}

/// `BonemealBehavior.isValidBonemealTarget`.
pub(crate) fn is_valid(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId) -> bool {
    let b = level.behaviours;
    let Some(kind) = kind(b, state) else {
        return false;
    };
    let above = off(pos, 0, 1, 0);
    match kind {
        Kind::Tall => {
            let name = b.table.name(state).to_string();
            let top = top_connected(level, pos, &name, 1);
            let bottom = top_connected(level, pos, &name, -1);
            (top.y - bottom.y).abs() < max_tall_height(pos.x, pos.z)
                && b.is_air(level.block_state(off(top, 0, 1, 0)))
        }
        Kind::Vine => {
            let bottom = top_connected(level, pos, "minecraft:vine", -1);
            b.is_air(level.block_state(off(bottom, 0, -1, 0)))
        }
        Kind::NetherWart => b.table.int(state, b.props.age) < 3,
        Kind::FruitStem => {
            b.table.has(state, b.props.age)
                && b.table.int(state, b.props.age) == 7
                && HORIZONTAL.iter().any(|&(dx, dz)| {
                    let side = off(pos, dx, 0, dz);
                    b.is_air(level.block_state(side)) && fruit_ground(b, level.block_state(off(side, 0, -1, 0)))
                })
        }
        Kind::Spread(..) | Kind::SporeBlossom => true,
        Kind::Coral => level.biome_name(pos) == Some("minecraft:warm_ocean"),
        Kind::ChorusFlower => chorus_flower_valid(level, state),
        Kind::ChorusPlant => chorus_flowers(level, pos)
            .into_iter()
            .any(|f| chorus_flower_valid(level, level.block_state(f))),
        Kind::Mycelium | Kind::Podzol => b.is_air(level.block_state(above)),
        Kind::Dirt => {
            // propagatesSkylightDown of the block above, read as "not a solid block".
            if b.is_solid(level.block_state(above)) {
                return false;
            }
            neighbours(level, pos).0 || neighbours(level, pos).1
        }
    }
}

/// Grass and mycelium in the 3×3×3 box around `pos`.
fn neighbours(level: &GameLevel<'_>, pos: BlockPos) -> (bool, bool) {
    let b = level.behaviours;
    let (mut grass, mut mycelium) = (false, false);
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let s = level.block_state(off(pos, dx, dy, dz));
                grass |= is(b, s, "minecraft:grass_block");
                mycelium |= is(b, s, "minecraft:mycelium");
            }
        }
    }
    (grass, mycelium)
}

/// `BonemealBehavior.isBonemealSuccess`: None for blocks this mod doesn't grow.
pub(crate) fn is_success(level: &mut GameLevel<'_>, _pos: BlockPos, state: BlockStateId) -> Option<bool> {
    let kind = kind(level.behaviours, state)?;
    Some(match kind {
        Kind::Coral => level.random().next_float() < 0.4,
        _ => true,
    })
}

/// `BonemealBehavior.performBonemeal`.
pub(crate) fn perform(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    let b = level.behaviours;
    let Some(kind) = kind(b, state) else {
        return;
    };
    match kind {
        Kind::Tall => {
            let name = b.table.name(state).to_string();
            let grown = b.table.default_state(b.block(state));
            let mut p = off(top_connected(level, pos, &name, 1), 0, 1, 0);
            let count = 1 + next_int(level, 2);
            for _ in 0..count {
                if !b.is_air(level.block_state(p)) {
                    break;
                }
                b.set_block(level, ticks, p, grown, 3);
                p = off(p, 0, 1, 0);
            }
        }
        Kind::Vine => {
            let grown = b.table.set_bool(state, b.props.up, false);
            let mut p = off(top_connected(level, pos, "minecraft:vine", -1), 0, -1, 0);
            // NetherVines.getBlocksToGrowWhenBonemealed.
            let mut probability = 1.0;
            let mut count = 0;
            while level.random().next_double() < probability {
                probability *= 0.826;
                count += 1;
            }
            for _ in 0..count {
                if !b.is_air(level.block_state(p)) {
                    break;
                }
                b.set_block(level, ticks, p, grown, 3);
                p = off(p, 0, -1, 0);
            }
        }
        Kind::NetherWart => {
            let age = b.table.int(state, b.props.age) + (2 + next_int(level, 4)) / 3;
            let grown = b.table.set_int(b.table.default_state(b.block(state)), b.props.age, age.min(3));
            b.set_block(level, ticks, pos, grown, 2);
        }
        Kind::FruitStem => {
            while level.block_state(pos) == state && next_int(level, 3) != 0 {
                b.random_tick(level, ticks, pos);
            }
        }
        Kind::Spread(width, most) => spread_away(level, ticks, pos, state, width, most),
        Kind::Coral => coral(level, ticks, pos, state),
        Kind::ChorusFlower => b.random_tick(level, ticks, pos),
        Kind::ChorusPlant => {
            for f in chorus_flowers(level, pos) {
                b.random_tick(level, ticks, f);
            }
        }
        Kind::Mycelium => {
            let (Some(brown), Some(red)) = (default(b, "minecraft:brown_mushroom"), default(b, "minecraft:red_mushroom")) else {
                return;
            };
            // SpreadAroundBehavior(width 3, height 1) from the block above.
            let centre = off(pos, 0, 1, 0);
            if centre.y < level.min_y() + 1 || centre.y + 1 >= level.max_y() {
                return;
            }
            for _ in 0..9 {
                let dx = next_int(level, 3) - next_int(level, 3);
                let dy = next_int(level, 1) - next_int(level, 1);
                let dz = next_int(level, 3) - next_int(level, 3);
                let p = off(centre, dx, dy, dz);
                let plant = if next_int(level, 2) == 0 { brown } else { red };
                if b.is_air(level.block_state(p)) && p.y > level.min_y() && b.can_survive(level, ticks, p, plant) {
                    b.set_block(level, ticks, p, plant, 2);
                }
            }
        }
        Kind::Dirt => {
            let (grass, mycelium) = neighbours(level, pos);
            let (Some(g), Some(m)) = (default(b, "minecraft:grass_block"), default(b, "minecraft:mycelium")) else {
                return;
            };
            let to = match (grass, mycelium) {
                (true, true) => {
                    if level.random().next_boolean() {
                        g
                    } else {
                        m
                    }
                }
                (true, false) => g,
                (false, true) => m,
                _ => return,
            };
            b.set_block(level, ticks, pos, to, 3);
        }
        Kind::Podzol => podzol(level, ticks, pos),
        Kind::SporeBlossom => level.emit(LevelEvent::DropResources { pos, state }),
    }
}

/// `SpreadAwayBehavior.placeOverworldGrass`.
fn spread_away(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, width: i32, most: i32) {
    let b = level.behaviours;
    let plant = b.table.default_state(b.block(state));
    let mut successes = 0;
    'tries: for i in (0..(width + 1) * 16).rev() {
        let mut p = pos;
        for _ in 0..i / 16 {
            let dx = next_int(level, 3) - 1;
            let dy = (next_int(level, 3) - 1) * next_int(level, 3) / 2;
            let dz = next_int(level, 3) - 1;
            p = off(p, dx, dy, dz);
            if !b.can_survive(level, ticks, p, plant) || b.is_solid(level.block_state(p)) {
                continue 'tries;
            }
        }
        if b.is_air(level.block_state(p)) && p.y > level.min_y() {
            b.set_block(level, ticks, p, plant, 2);
            successes += 1;
            if successes >= most {
                return;
            }
        }
    }
}

/// `PodzolBehavior.performBonemeal`: ferns (some grown into large ferns), sweet berry bushes and
/// the odd dead bush on the podzol around.
fn podzol(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos) {
    let b = level.behaviours;
    let (Some(fern), Some(bush), Some(dead)) = (
        default(b, "minecraft:fern"),
        default(b, "minecraft:sweet_berry_bush"),
        default(b, "minecraft:dead_bush"),
    ) else {
        return;
    };
    let source = off(pos, 0, 1, 0);
    'tries: for i in 0..128 {
        let mut p = source;
        for _ in 0..i / 16 {
            let dx = next_int(level, 3) - 1;
            let dy = (next_int(level, 3) - 1) * next_int(level, 3) / 2;
            let dz = next_int(level, 3) - 1;
            p = off(p, dx, dy, dz);
            if !is(b, level.block_state(off(p, 0, -1, 0)), "minecraft:podzol") || b.is_solid(level.block_state(p)) {
                continue 'tries;
            }
        }
        let here = level.block_state(p);
        if is(b, here, "minecraft:fern") && next_int(level, 10) == 0 {
            // The fern's own performBonemeal: a large fern.
            b.grow_crop(level, ticks, p);
        }
        if b.is_air(here) && next_int(level, 5) == 0 && b.is_air(level.block_state(p)) && p.y > level.min_y() {
            // Weighted: bush age 3 ×2, age 2 ×4, age 1 ×8, age 0 ×12, fern ×120, dead bush ×1.
            let roll = next_int(level, 147);
            let place = match roll {
                0..2 => b.table.set_int(bush, b.props.age, 3),
                2..6 => b.table.set_int(bush, b.props.age, 2),
                6..14 => b.table.set_int(bush, b.props.age, 1),
                14..26 => bush,
                26..146 => fern,
                _ => dead,
            };
            b.set_block(level, ticks, p, place, 2);
        }
    }
}

fn coral_replaceable(b: &Behaviours, state: BlockStateId) -> bool {
    is(b, state, "minecraft:water")
        || in_tag(b, state, "minecraft:coral_blocks")
        || in_tag(b, state, "minecraft:coral_plants")
        || in_tag(b, state, "minecraft:wall_corals")
}

/// `CoralBehavior.place`: the coral becomes a coral tree of its type's coral block.
fn coral(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    let b = level.behaviours;
    let name = b.table.name(state).to_string();
    let block = name
        .strip_prefix("minecraft:")
        .and_then(|n| n.find("_coral").map(|i| n[..i].to_string()))
        .filter(|t| CORAL_TYPES.contains(&t.as_str()))
        .unwrap_or_else(|| CORAL_TYPES[next_int(level, CORAL_TYPES.len() as i32) as usize].to_string());
    let Some(coral_block) = default(b, &format!("minecraft:{block}_coral_block")) else {
        return;
    };
    b.remove_block(level, ticks, pos, false);
    if !coral_tree(level, ticks, pos, coral_block) {
        b.set_block(level, ticks, pos, state, 3);
    }
}

/// `CoralBehavior.placeFeature`, CoralTreeFeature's shape.
fn coral_tree(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, block: BlockStateId) -> bool {
    let b = level.behaviours;
    let trunk = next_int(level, 3) + 1;
    // isValidPosition: open water (or coral) above, widening.
    if pos.y < level.min_y() + 1 || pos.y + trunk + 1 >= level.max_y() {
        return false;
    }
    for j in 0..=trunk + 4 {
        let k = if j < trunk { 0 } else { (j - trunk) / 2 + 1 };
        for l in -k..=k {
            for m in -k..=k {
                if !coral_replaceable(b, level.block_state(off(pos, l, j, m))) {
                    return false;
                }
            }
        }
    }
    let mut p = pos;
    for j in 0..trunk {
        if !coral_block(level, ticks, p, block, j == trunk - 1) {
            return j != 0;
        }
        p = off(p, 0, 1, 0);
    }
    let top = p;
    let arms = next_int(level, 3) + 2;
    let mut dirs = HORIZONTAL.to_vec();
    for i in (1..dirs.len()).rev() {
        let j = next_int(level, i as i32 + 1) as usize;
        dirs.swap(i, j);
    }
    for &(dx, dz) in &dirs[..arms as usize] {
        let mut q = off(top, dx, 0, dz);
        let length = next_int(level, 5) + 2;
        let mut since = 0;
        for step in 0..length {
            if !coral_block(level, ticks, q, block, true) {
                break;
            }
            since += 1;
            q = off(q, 0, 1, 0);
            if step == 0 || since >= 2 && level.random().next_float() < 0.25 {
                q = off(q, dx, 0, dz);
                since = 0;
            }
        }
    }
    true
}

/// `CoralBehavior.placeCoralBlock`: the block, maybe a coral or sea pickle on top, and wall fans.
fn coral_block(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, block: BlockStateId, decorate: bool) -> bool {
    let b = level.behaviours;
    let above = off(pos, 0, 1, 0);
    if !coral_replaceable(b, level.block_state(pos)) || !coral_replaceable(b, level.block_state(above)) {
        return false;
    }
    b.set_block(level, ticks, pos, block, 3);
    if decorate {
        if level.random().next_float() < 0.25 {
            let t = CORAL_TYPES[next_int(level, CORAL_TYPES.len() as i32) as usize];
            if let Some(plant) = default(b, &format!("minecraft:{t}_coral")) {
                b.set_block(level, ticks, above, plant, 2);
            }
        } else if level.random().next_float() < 0.05 {
            if let Some(pickle) = default(b, "minecraft:sea_pickle") {
                let n = next_int(level, 4) + 1;
                b.set_block(level, ticks, above, b.table.set_int(pickle, b.props.pickles, n), 2);
            }
        }
    }
    for (dx, dz, facing) in [(0, -1, "north"), (1, 0, "east"), (0, 1, "south"), (-1, 0, "west")] {
        if level.random().next_float() < 0.2 {
            let side = off(pos, dx, 0, dz);
            if is(b, level.block_state(side), "minecraft:water") {
                let t = CORAL_TYPES[next_int(level, CORAL_TYPES.len() as i32) as usize];
                if let Some(fan) = default(b, &format!("minecraft:{t}_coral_wall_fan")) {
                    let fan = b.table.with_text(fan, b.props.facing, facing).unwrap_or(fan);
                    b.set_block(level, ticks, side, fan, 2);
                }
            }
        }
    }
    true
}
