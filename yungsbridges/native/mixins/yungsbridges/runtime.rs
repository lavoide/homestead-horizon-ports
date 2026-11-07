// YUNG's Bridges 4.0.3 (YungNickYoung, LGPLv3): bridges over rivers. On Java they're a feature in
// the river biomes' surface_structures step (bridge_list, a multiple_attempt_single_random of 22
// bridges), with a placement modifier that looks for a river crossing (BridgePlacement) and
// twelve processors that run over the placed template (BridgeFeature.PROCESSORS). Horizon has
// none of those Java types, so this places them after generation: a newly generated overworld
// chunk is queued, and once the chunks around it are loaded the same scan, rarity and template
// run there. The templates and the list are bridge_data.rs (tools/yungsbridges_port.py).
//
// Injected as `crate::game::yungsbridges`.
use super::*;
use super::structure_blocks::TemplateEntry;
use super::yungsbridges_data::{BRIDGES, Bridge, TEMPLATES, Template};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

/// Level.getSeaLevel() in the overworld.
const SEA_LEVEL: i32 = 63;
/// Queued chunks looked at per tick, and the most kept.
const PER_TICK: usize = 64;
const MAX_QUEUE: usize = 8192;

thread_local! {
    static QUEUE: RefCell<VecDeque<[i32; 2]>> = RefCell::new(VecDeque::new());
    static ENTRIES: RefCell<HashMap<usize, Option<TemplateEntry>>> = RefCell::new(HashMap::new());
    /// With YUNGSBRIDGES_LOG: chunks queued, decorated, in a river, crossings, past the rarity.
    static STATS: RefCell<[u32; 7]> = const { RefCell::new([0; 7]) };
    /// YUNGSBRIDGES_FORCE="cx,cz;..." (testing): those chunks are queued once, and the biome
    /// filter and rarity are skipped.
    static FORCED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

fn forced() -> bool {
    FORCED.get().unwrap_or(false)
}

fn stat(i: usize) {
    if std::env::var_os("YUNGSBRIDGES_LOG").is_none() {
        return;
    }
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s[i] += 1;
        if i == 1 && s[1] % 100 == 0 {
            eprintln!("yungsbridges stats: queued {} decorated {} river {} crossings {} placed {} (opaque starts {}, banks {})", s[0], s[1], s[2], s[3], s[4], s[5], s[6]);
        }
    });
}

/// A chunk came into the world: a new overworld one waits for its bridge.
pub(crate) fn chunk_loaded(dimension: &str, chunk: [i32; 2], new: bool) {
    if !new || dimension != "minecraft:overworld" {
        return;
    }
    QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        if q.len() < MAX_QUEUE {
            q.push_back(chunk);
            stat(0);
        }
    });
}

/// Each server tick: the queued chunks whose surroundings are loaded get their chance.
pub(crate) fn tick(game: &mut Game<'_>) {
    if game.dim.id() != "minecraft:overworld" {
        return;
    }
    if FORCED.get().is_none() {
        let list = std::env::var("YUNGSBRIDGES_FORCE").unwrap_or_default();
        FORCED.set(Some(!list.is_empty()));
        // Once per process, not per thread: FORCED is thread-local.
        static TAKEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let first = !TAKEN.swap(true, std::sync::atomic::Ordering::Relaxed);
        for pair in list.split(';').filter(|p| first && !p.is_empty()) {
            if let Some((x, z)) = pair.split_once(',')
                && let (Ok(x), Ok(z)) = (x.trim().parse(), z.trim().parse())
            {
                QUEUE.with(|q| q.borrow_mut().push_front([x, z]));
            }
        }
    }
    let n = QUEUE.with(|q| q.borrow().len()).min(PER_TICK);
    for _ in 0..n {
        let Some(chunk) = QUEUE.with(|q| q.borrow_mut().pop_front()) else {
            return;
        };
        // The scan reaches up to 16 + 31 + 1 blocks past the corner, the template and its legs
        // a few more: the chunks from one before to three after on each axis.
        let ready = (chunk[0] - 1..=chunk[0] + 3)
            .all(|x| (chunk[1] - 1..=chunk[1] + 3).all(|z| game.world.chunks.contains_key(&(x, z))));
        if !ready {
            QUEUE.with(|q| q.borrow_mut().push_back(chunk));
            continue;
        }
        decorate(game, chunk);
    }
}

fn facts(game: &Game<'_>, p: [i32; 3]) -> horizon_client::blocks::StateFacts {
    game.res.facts().get(game.world.block(p[0], p[1], p[2])).clone()
}

fn name_at(game: &Game<'_>, p: [i32; 3]) -> String {
    facts(game, p).name.to_string()
}

/// BlockState.isOpaque (yarn; canOcclude).
fn opaque(game: &Game<'_>, p: [i32; 3]) -> bool {
    facts(game, p).can_occlude
}

/// BlockState.isLiquid. Java looks before the vegetal decoration step; here the river already
/// has its kelp and seagrass, so a block holding water counts too.
fn liquid(game: &Game<'_>, p: [i32; 3]) -> bool {
    facts(game, p).fluid_level > 0
}

/// What generatePillarDown goes down through: air and liquid, and the plants decoration put
/// there since (Java's legs went in before them).
fn open(game: &Game<'_>, p: [i32; 3]) -> bool {
    let f = facts(game, p);
    f.fluid_level > 0 || f.replaceable || air(game, p)
}

fn air(game: &Game<'_>, p: [i32; 3]) -> bool {
    matches!(name_at(game, p).as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

/// `getHeight(WORLD_SURFACE, x, z) <= sea level` as it was when Java looked, before the
/// vegetal decoration: nothing above `y` but air and what that step adds (plants, logs, leaves).
fn open_sky(game: &Game<'_>, x: i32, y: i32, z: i32) -> bool {
    (y + 1..y + 49).all(|yy| {
        let f = facts(game, [x, yy, z]);
        let name = &*f.name;
        f.replaceable
            || air(game, [x, yy, z])
            || game.block_in_tag("minecraft:logs", name)
            || game.block_in_tag("minecraft:leaves", name)
            || f.classes.iter().any(|c| &**c == "VegetationBlock")
    })
}

fn biome(game: &Game<'_>, p: [i32; 3]) -> String {
    let at = Vec3::new(f64::from(p[0]) + 0.5, f64::from(p[1]) + 0.5, f64::from(p[2]) + 0.5);
    game.biome_name_at(at).unwrap_or_default()
}

/// The river biomes (#minecraft:is_river, the bridges' has_structure tag).
fn river(name: &str) -> bool {
    matches!(name, "minecraft:river" | "minecraft:frozen_river")
}

/// ChunkGenerator.applyBiomeDecoration for bridge_list: the biome filter at the chunk's
/// origin, then MultipleAttemptSingleRandomFeature: the bridges in random order until one
/// places.
fn decorate(game: &mut Game<'_>, chunk: [i32; 2]) {
    stat(1);
    let origin = [chunk[0] * 16, game.world.shape.min_y, chunk[1] * 16];
    if !forced() && !river(&biome(game, origin)) {
        return;
    }
    stat(2);
    let mut left: Vec<usize> = (0..BRIDGES.len()).collect();
    while !left.is_empty() {
        let i = game.world.random.next_int_bound(left.len() as i32).unwrap_or(0) as usize;
        if try_bridge(game, &BRIDGES[left[i]], origin) {
            return;
        }
        left.remove(i);
    }
}

/// One placed feature: BridgePlacement, then the rarity filter, then BridgeFeature.
fn try_bridge(game: &mut Game<'_>, bridge: &Bridge, origin: [i32; 3]) -> bool {
    let Some(at) = crossing(game, bridge, origin) else {
        return false;
    };
    stat(3);
    // RarityFilter: 1 in `chance`.
    if !forced() && game.world.random.next_float() >= 1.0 / bridge.rarity as f32 {
        return false;
    }
    stat(4);
    place(game, bridge, at)
}

/// BridgePlacement.getPositions: a crossing in this chunk, solid ground at both ends (the
/// banks, at sea level - 1, open to the sky above sea level) and water under the whole span.
fn crossing(game: &Game<'_>, b: &Bridge, origin: [i32; 3]) -> Option<[i32; 3]> {
    let sea = SEA_LEVEL - 1;
    let along = |p: [i32; 3], minor: i32, major: i32| {
        if b.z_axis { [p[0] + minor, sea, p[2] + major] } else { [p[0] + major, sea, p[2] + minor] }
    };
    let bank = |p: [i32; 3]| opaque(game, p) && open_sky(game, p[0], p[1], p[2]);
    let solid_run = |end: [i32; 3]| {
        let mut n = 1;
        for direction in [-1, 1] {
            for d in 1..=b.width / 2 {
                if !bank(along(end, direction * d, 0)) {
                    break;
                }
                n += 1;
            }
        }
        n
    };
    let half = b.width / 2;
    for minor in (half + b.width_offset + 1)..(16 - half - b.width_offset) {
        for major in 0..16 {
            let start = along([origin[0], sea, origin[2]], minor, major);
            let end = along(start, 0, b.length + 1);
            if opaque(game, start) {
                stat(5);
            }
            if !bank(start) || !bank(end) {
                continue;
            }
            stat(6);
            if solid_run(start) < b.solid_needed || solid_run(end) < b.solid_needed {
                continue;
            }
            let water = (-half..=half)
                .all(|m| (b.min_water..=b.max_water).all(|w| liquid(game, along(start, m, w))));
            if water {
                return Some(if b.z_axis {
                    [start[0] - half - b.width_offset, sea, start[2] + 1]
                } else {
                    [start[0] + 1, sea, start[2] + half + b.width_offset]
                });
            }
        }
    }
    None
}

fn entry(index: usize, template: &Template) -> Option<TemplateEntry> {
    ENTRIES.with(|e| {
        e.borrow_mut()
            .entry(index)
            .or_insert_with(|| {
                let raw = horizon_world::compression::Compression::Gzip.decode(template.nbt, 1 << 28).ok()?;
                let (named, _) =
                    horizon_world::nbt::read_named(&raw, horizon_world::nbt::Limits::templates()).ok()?;
                Some(TemplateEntry { tag: named.tag, author: "?".to_owned() })
            })
            .clone()
    })
}

/// StructurePlaceSettings' rotation about the template's origin (no pivot is set):
/// COUNTERCLOCKWISE_90 takes (x, z) to (z, -x).
fn rotated(corner: [i32; 3], p: [i32; 3], z_axis: bool) -> [i32; 3] {
    if z_axis {
        [corner[0] + p[0], corner[1] + p[1], corner[2] + p[2]]
    } else {
        [corner[0] + p[2], corner[1] + p[1], corner[2] - p[0]]
    }
}

/// BridgeFeature.place: the template at sea level, rotated for an x-axis bridge, then the
/// processors in their order over its marker blocks.
fn place(game: &mut Game<'_>, b: &Bridge, at: [i32; 3]) -> bool {
    let template = &TEMPLATES[b.template];
    let Some(entry) = entry(b.template, template) else {
        return false;
    };
    use horizon_world::worldgen::structure::{Mirror, Rotation};
    let rotation = if b.z_axis { Rotation::None } else { Rotation::Counterclockwise90 };
    let corner = [at[0], SEA_LEVEL, at[2]];
    let seed = i64::from(game.world.random.next_int());
    let settings = super::structure_blocks::load_settings(Mirror::None, rotation, 1.0, seed, false);
    let pos = horizon_core::position::BlockPos::new(corner[0], corner[1], corner[2]);
    if !game.place_template_entry(&entry, pos, settings, Mirror::None, rotation) {
        return false;
    }
    // The markers as placed (rotated), before any processor changes them: StructureBlockInfo.
    let reg = game.pack.assets.registry;
    let infos: Vec<([i32; 3], &str, Option<horizon_world::chunk::BlockState>)> = template
        .markers
        .iter()
        .map(|(p, name)| {
            let w = rotated(corner, *p, b.z_axis);
            (w, *name, reg.state(game.world.block(w[0], w[1], w[2])).cloned())
        })
        .collect();
    let biome = biome(game, corner);
    Processors { game: &mut *game, biome: &biome }.run(&infos);
    if std::env::var_os("YUNGSBRIDGES_LOG").is_some() {
        eprintln!("yungsbridges: {} at {:?} (z axis {})", template.id, corner, b.z_axis);
    }
    true
}

struct Processors<'a, 'g> {
    game: &'a mut Game<'g>,
    biome: &'a str,
}

/// Which wood ITemplateFeatureProcessor's biome variants pick: the mod's collections tags
/// (badlands, taiga, jungle, savanna; vanilla members only), oak otherwise.
fn wood(biome: &str) -> &'static str {
    match biome {
        "minecraft:badlands" | "minecraft:eroded_badlands" | "minecraft:wooded_badlands" => "dark_oak",
        "minecraft:taiga" => "spruce",
        "minecraft:jungle" | "minecraft:sparse_jungle" | "minecraft:bamboo_jungle" => "jungle",
        "minecraft:savanna" | "minecraft:savanna_plateau" | "minecraft:windswept_savanna" => "acacia",
        _ => "oak",
    }
}

const STAIRS: &[(&str, &str)] = &[("facing", "north"), ("half", "bottom"), ("shape", "straight"), ("waterlogged", "false")];
const SLAB: &[(&str, &str)] = &[("type", "bottom"), ("waterlogged", "false")];
const WALL: &[(&str, &str)] = &[("east", "none"), ("north", "none"), ("south", "none"), ("west", "none"), ("up", "true"), ("waterlogged", "false")];
const FENCE: &[(&str, &str)] = &[("east", "false"), ("north", "false"), ("south", "false"), ("west", "false"), ("waterlogged", "false")];
const LOG: &[(&str, &str)] = &[("axis", "y")];
const LANTERN: &[(&str, &str)] = &[("hanging", "true"), ("waterlogged", "true")];

impl Processors<'_, '_> {
    fn random(&mut self) -> f32 {
        self.game.world.random.next_float()
    }

    /// BlockStateRandomizer.get: the first entry whose share of [0, 1) the roll lands in, else
    /// the default.
    fn pick(&mut self, default: &'static str, entries: &[(&'static str, f32)]) -> &'static str {
        let roll = self.random();
        let mut bottom = 0.0;
        for (name, share) in entries {
            if roll >= bottom && roll < bottom + share {
                return name;
            }
            bottom += share;
        }
        default
    }

    /// `name` with the listed properties copied from `source` (each processor's
    /// get<Kind>BlockWithState: the template's value, else its default), when the block has them.
    fn state(&self, name: &str, source: Option<&horizon_world::chunk::BlockState>, keys: &[(&str, &str)]) -> Option<BlockStateId> {
        let name = if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") };
        let reg = self.game.pack.assets.registry;
        let mut state = reg.state(reg.default_state(&name)?)?.clone();
        for (key, default) in keys {
            if state.properties.contains_key(*key) {
                let value = source
                    .and_then(|s| s.properties.get(*key).cloned())
                    .unwrap_or_else(|| (*default).into());
                state.properties.insert((*key).into(), value);
            }
        }
        reg.resolve(&state).ok()
    }

    fn set(&mut self, p: [i32; 3], state: Option<BlockStateId>) {
        if let Some(id) = state {
            // Level.setBlock(.., 2): to the clients, no neighbour updates.
            let _ = self.game.level_set_with_flags(p, id, 2);
        }
    }

    fn put(&mut self, p: [i32; 3], name: &str, source: Option<&horizon_world::chunk::BlockState>, keys: &[(&str, &str)]) {
        let state = self.state(name, source, keys);
        self.set(p, state);
    }

    /// ITemplateFeatureProcessor.generatePillarDown: the replacement at `p`, then legs down
    /// through air and liquid (and plants, see `open`), to y 0 as on Java.
    fn pillar(&mut self, p: [i32; 3], mut replacement: impl FnMut(&mut Self) -> Option<BlockStateId>, mut leg: impl FnMut(&mut Self) -> Option<BlockStateId>) {
        let r = replacement(self);
        self.set(p, r);
        let mut q = [p[0], p[1] - 1, p[2]];
        while q[1] > 0 && open(self.game, q) {
            let l = leg(self);
            self.set(q, l);
            q[1] -= 1;
        }
    }

    fn stone_brick(&mut self) -> &'static str {
        self.pick("stone_bricks", &[("mossy_stone_bricks", 0.5), ("cracked_stone_bricks", 0.25)])
    }

    fn cobble(&mut self) -> &'static str {
        self.pick("cobblestone", &[("mossy_cobblestone", 0.6)])
    }

    fn run(&mut self, infos: &[([i32; 3], &str, Option<horizon_world::chunk::BlockState>)]) {
        let of = |block: &str| -> Vec<([i32; 3], Option<horizon_world::chunk::BlockState>)> {
            let want = format!("minecraft:{block}");
            infos.iter().filter(|(_, n, _)| *n == want).map(|(p, _, s)| (*p, s.clone())).collect()
        };
        let wood = wood(self.biome);
        let log = format!("{wood}_log");
        let fence = format!("{wood}_fence");
        let planks = format!("{wood}_planks");
        let slab = format!("{wood}_slab");
        let stairs = format!("{wood}_stairs");
        // DynamicLegProcessor.
        for (p, s) in of("yellow_stained_glass") {
            self.pillar(p, |me| me.state(&log, s.as_ref(), LOG), |me| me.state(&log, s.as_ref(), LOG));
        }
        for (p, s) in of("warped_fence") {
            self.pillar(p, |me| me.state(&fence, s.as_ref(), FENCE), |me| me.state(&fence, s.as_ref(), FENCE));
        }
        for (p, _) in of("pink_stained_glass") {
            self.pillar(p, |me| { let n = me.stone_brick(); me.state(n, None, &[]) }, |me| { let n = me.stone_brick(); me.state(n, None, &[]) });
        }
        for (p, _) in of("light_blue_stained_glass") {
            self.pillar(p, |me| me.state("polished_andesite", None, &[]), |me| me.state("polished_andesite", None, &[]));
        }
        for (p, _) in of("magenta_stained_glass") {
            self.pillar(p, |me| { let n = me.cobble(); me.state(n, None, &[]) }, |me| { let n = me.cobble(); me.state(n, None, &[]) });
        }
        for (p, _) in of("gray_stained_glass") {
            self.pillar(p, |me| me.state("polished_andesite", None, &[]), |me| { let n = me.stone_brick(); me.state(n, None, &[]) });
        }
        for (p, s) in of("prismarine_wall") {
            // The leg is getFenceBlockWithState of a wall: the plain cobblestone wall.
            self.pillar(p, |me| me.state("cobblestone_wall", s.as_ref(), WALL), |me| me.state("cobblestone_wall", None, &[]));
        }
        // LogBiomeProcessor, StairBiomeProcessor, PlanksBiomeProcessor, SlabBiomeProcessor.
        for (p, s) in of("oak_log") { self.put(p, &log, s.as_ref(), LOG); }
        for (p, s) in of("oak_stairs") { self.put(p, &stairs, s.as_ref(), STAIRS); }
        for (p, _) in of("oak_planks") { self.put(p, &planks, None, &[]); }
        for (p, s) in of("oak_slab") { self.put(p, &slab, s.as_ref(), SLAB); }
        // StoneVariationProcessor.
        for (p, _) in of("stone_bricks") { let n = self.stone_brick(); self.put(p, n, None, &[]); }
        for (p, s) in of("stone_brick_slab") { let n = self.pick("stone_brick_slab", &[("mossy_stone_brick_slab", 0.5)]); self.put(p, n, s.as_ref(), SLAB); }
        for (p, s) in of("stone_brick_wall") { let n = self.pick("stone_brick_wall", &[("mossy_stone_brick_wall", 0.5)]); self.put(p, n, s.as_ref(), WALL); }
        for (p, s) in of("stone_brick_stairs") { let n = self.pick("stone_brick_stairs", &[("mossy_stone_brick_stairs", 0.5)]); self.put(p, n, s.as_ref(), STAIRS); }
        for (p, _) in of("cobblestone") { let n = self.cobble(); self.put(p, n, None, &[]); }
        // LanternRotProcessor (half go). Java puts the wall torch back in its default facing;
        // here it keeps the template's.
        for (p, s) in of("lantern") { let n = self.pick("air", &[("lantern", 0.5)]); self.put(p, n, s.as_ref(), LANTERN); }
        for (p, _) in of("torch") { let n = self.pick("air", &[("torch", 0.5)]); self.put(p, n, None, &[]); }
        for (p, s) in of("wall_torch") { let n = self.pick("air", &[("wall_torch", 0.5)]); self.put(p, n, s.as_ref(), &[("facing", "north")]); }
        // OptionalWallProcessor.
        for (p, s) in of("end_stone_brick_wall") { let n = self.pick("air", &[("stone_brick_wall", 0.5)]); self.put(p, n, s.as_ref(), WALL); }
        // OptionalBlockProcessor.
        for (p, _) in of("brown_stained_glass") { let n = self.pick("air", &[("stone_bricks", 0.1875), ("mossy_stone_bricks", 0.375), ("cracked_stone_bricks", 0.1875)]); self.put(p, n, None, &[]); }
        for (p, _) in of("blue_stained_glass") { let n = self.pick("air", &[("chiseled_stone_bricks", 0.75)]); self.put(p, n, None, &[]); }
        for (p, _) in of("red_stained_glass") { let n = self.pick("air", &[("cobblestone", 0.75)]); self.put(p, n, None, &[]); }
        for (p, _) in of("end_stone_bricks") { let n = self.pick("air", &[("polished_andesite", 0.75)]); self.put(p, n, None, &[]); }
        for (p, _) in of("dark_prismarine") { let n = self.pick("air", &[("cobblestone", 0.375), ("andesite", 0.375)]); self.put(p, n, None, &[]); }
        for (p, s) in of("lime_stained_glass") {
            if self.random() < 0.5 { self.put(p, &log, s.as_ref(), LOG); } else { self.put(p, "air", None, &[]); }
        }
        // OptionalSlabProcessor.
        for (p, s) in of("purpur_slab") { let n = self.pick("air", &[("stone_brick_slab", 0.75)]); self.put(p, n, s.as_ref(), SLAB); }
        for (p, s) in of("end_stone_brick_slab") { let n = self.pick("air", &[("polished_andesite_slab", 0.75)]); self.put(p, n, s.as_ref(), SLAB); }
        for (p, s) in of("dark_prismarine_slab") { let n = self.pick("air", &[("cobblestone_slab", 0.45), ("andesite_slab", 0.45)]); self.put(p, n, s.as_ref(), SLAB); }
        for (p, s) in of("prismarine_brick_slab") { let n = self.pick("air", &[("smooth_stone_slab", 0.9)]); self.put(p, n, s.as_ref(), SLAB); }
        for (p, s) in of("crimson_slab") {
            if self.random() < 0.5 { self.put(p, &slab, s.as_ref(), SLAB); } else { self.put(p, "air", None, &[]); }
        }
        for (p, s) in of("warped_slab") { let n = self.pick("air", &[("cobblestone_slab", 0.9)]); self.put(p, n, s.as_ref(), SLAB); }
        // OptionalStairProcessor.
        for (p, s) in of("purpur_stairs") { let n = self.pick("air", &[("stone_brick_stairs", 0.75)]); self.put(p, n, s.as_ref(), STAIRS); }
        for (p, s) in of("dark_prismarine_stairs") { let n = self.pick("air", &[("cobblestone_stairs", 0.375), ("andesite_stairs", 0.375)]); self.put(p, n, s.as_ref(), STAIRS); }
        for (p, s) in of("warped_stairs") { let n = self.pick("air", &[("cobblestone_stairs", 0.75)]); self.put(p, n, s.as_ref(), STAIRS); }
        for (p, s) in of("crimson_stairs") { self.put(p, &stairs, s.as_ref(), STAIRS); }
        // FenceBiomeProcessor: a quarter of the fences with nothing opaque above go, unhooking
        // their north and west neighbours; the rest take the biome's wood and drop their north
        // and west arms where nothing opaque is there.
        for (p, s) in of("oak_fence") {
            let above = [p[0], p[1] + 1, p[2]];
            let north = [p[0], p[1], p[2] - 1];
            let west = [p[0] - 1, p[1], p[2]];
            if self.random() >= 0.75 && !opaque(self.game, above) {
                self.put(p, "air", None, &[]);
                self.unhook(north, "south");
                self.unhook(west, "east");
            } else {
                let north_open = !opaque(self.game, north);
                let west_open = !opaque(self.game, west);
                let reg = self.game.pack.assets.registry;
                let Some(mut state) = self.state(&fence, s.as_ref(), FENCE).and_then(|id| reg.state(id).cloned()) else { continue };
                if north_open { state.properties.insert("north".into(), "false".into()); }
                if west_open { state.properties.insert("west".into(), "false".into()); }
                let id = reg.resolve(&state).ok();
                self.set(p, id);
            }
        }
    }

    /// Sets a fence's `side` arm false, if it has it on.
    fn unhook(&mut self, p: [i32; 3], side: &str) {
        let reg = self.game.pack.assets.registry;
        let Some(mut state) = reg.state(self.game.world.block(p[0], p[1], p[2])).cloned() else { return };
        if state.properties.get(side).is_some_and(|v| v == "true") {
            state.properties.insert(side.into(), "false".into());
            let id = reg.resolve(&state).ok();
            self.set(p, id);
        }
    }
}
