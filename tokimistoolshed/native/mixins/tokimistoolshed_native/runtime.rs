// Tokimi's Toolshed 1.8 (Cozy Studios): what the tools do. The items are the content mod
// mods/tokimistoolshed (tools/tokimistoolshed_port.py).
// - HammerItem, ExcavatorItem, ScytheItem, ClippersItem.postMine: the 3x3 around a block a
//   survival player broke, in the plane of the face they hit (LastBreakData), unless sneaking;
// - LumberAxeItem.postMine: the whole tree;
// - ScytheItem.useOnBlock: the 3x3 tilled;
// - the copper buckets (CopperBucketItem, CopperBucketBehaviors, CowMilkMixin,
//   CopperMilkBucketItem): a vanilla bucket's uses with 16 uses of wear;
// - ShearsLootMixin: blocks drop for clippers what they drop for shears;
// - TrowelItem, ChiselItem, AbacusItem.
//
// Injected as `crate::game::tokimistoolshed`.
use super::*;
use core::ops::ControlFlow;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};

const NS: &str = "tokimistoolshed:";
const COPPER_BUCKET: &str = "tokimistoolshed:copper_bucket";
const COPPER_MILK: &str = "tokimistoolshed:copper_milk_bucket";
/// CopperBucketItem's maxDamage: 16 uses.
const BUCKET_USES: u32 = 16;
/// LumberAxeItem.MAX_TREE_SIZE and MIN_LEAVES_REQUIRED.
const MAX_TREE: usize = 256;
const MIN_LEAVES: usize = 3;
/// Homestead has Falling Trees (native/fallingtrees). On Java its BlockEvent.BREAK takes the
/// whole tree for any axe (LumberAxeItem is one) and cancels the break, so the lumber axe's
/// postMine never runs; native/fallingtrees fells it on the next tick instead. Set this to
/// false if Falling Trees is ever left out (a mixin can't ask which native mods are loaded:
/// mod_api.rs takes no modules).
const FALLING_TREES: bool = true;
/// AbacusHudRenderer.MAX_AXIS_LENGTH.
const MAX_AXIS: i32 = 128;

thread_local! {
    // The static guards of the Java items (isHammering, isLumbering...): the hooks call the
    // function they hook again for the same action.
    static BREAKING: Cell<bool> = const { Cell::new(false) };
    static TILLING: Cell<bool> = const { Cell::new(false) };
    static BUCKET: Cell<bool> = const { Cell::new(false) };
    static HAND: Cell<bool> = const { Cell::new(false) };
    static LOOT: Cell<bool> = const { Cell::new(false) };
    /// LastBreakData: the face each block was last hit on.
    static FACES: RefCell<HashMap<[i32; 3], [i32; 3]>> = RefCell::new(HashMap::new());
    static HUD_TICK: Cell<u32> = const { Cell::new(0) };
    /// A toolshed item's use on a block that acted: (player, game time). See `swallowed`.
    static USED_ON_BLOCK: Cell<Option<(u128, i64)>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Excavator,
    Hammer,
    Scythe,
    LumberAxe,
    Clippers,
}

fn kind(item: &str) -> Option<Kind> {
    let name = item.strip_prefix(NS)?;
    Some(if name == "clippers" {
        Kind::Clippers
    } else if name.ends_with("_excavator") {
        Kind::Excavator
    } else if name.ends_with("_hammer") {
        Kind::Hammer
    } else if name.ends_with("_scythe") {
        Kind::Scythe
    } else if name.ends_with("_lumber_axe") {
        Kind::LumberAxe
    } else {
        return None;
    })
}

fn sneaking(game: &Game<'_>) -> bool {
    game.keys.contains(&KeyCode::ShiftLeft)
}

fn state_at(game: &Game<'_>, p: [i32; 3]) -> BlockStateId {
    game.world.block(p[0], p[1], p[2])
}

fn facts_at(game: &Game<'_>, p: [i32; 3]) -> horizon_client::blocks::StateFacts {
    game.res.facts().get(state_at(game, p)).clone()
}

fn is_air(name: &str) -> bool {
    matches!(name, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

fn has_class(facts: &horizon_client::blocks::StateFacts, class: &str) -> bool {
    facts.classes.iter().any(|c| &**c == class)
}

/// `instanceof PlantBlock` (26.3's VegetationBlock).
fn plant(facts: &horizon_client::blocks::StateFacts) -> bool {
    has_class(facts, "VegetationBlock") || has_class(facts, "BushBlock")
}

/// ClippersItem.isClippable.
fn clippable(game: &Game<'_>, facts: &horizon_client::blocks::StateFacts) -> bool {
    let name = &*facts.name;
    game.block_in_tag("minecraft:leaves", name)
        || game.block_in_tag("minecraft:wool", name)
        || (plant(facts) && !has_class(facts, "CropBlock"))
        || matches!(
            name,
            "minecraft:cobweb"
                | "minecraft:moss_block"
                | "minecraft:moss_carpet"
                | "minecraft:vine"
                | "minecraft:glow_lichen"
                | "minecraft:hanging_roots"
                | "minecraft:nether_sprouts"
        )
}

/// The block postMine starts from: the kind's own blocks.
fn takes(game: &Game<'_>, kind: Kind, facts: &horizon_client::blocks::StateFacts) -> bool {
    let name = &*facts.name;
    match kind {
        Kind::Excavator => game.block_in_tag("minecraft:mineable/shovel", name),
        Kind::Hammer => game.block_in_tag("minecraft:mineable/pickaxe", name),
        Kind::Scythe => game.block_in_tag("minecraft:mineable/hoe", name) || plant(facts),
        Kind::LumberAxe => game.block_in_tag("minecraft:logs", name),
        Kind::Clippers => clippable(game, facts),
    }
}

/// HammerItem.getOffsetPos: the 3x3 in the plane of the face that was hit.
fn offset(origin: [i32; 3], face: [i32; 3], a: i32, b: i32) -> [i32; 3] {
    match face {
        [0, _, 0] => [origin[0] + a, origin[1], origin[2] + b],
        [0, 0, _] => [origin[0] + a, origin[1] + b, origin[2]],
        _ => [origin[0], origin[1] + b, origin[2] + a],
    }
}

/// LastBreakData.setFace, from the attack callback.
pub(crate) fn hit_face(at: [i32; 3], face: [i32; 3]) {
    FACES.with(|f| {
        let mut f = f.borrow_mut();
        if f.len() > 256 {
            f.clear();
        }
        f.insert(at, face);
    });
}

/// `Game::break_block` for a player holding one of the area tools. The block breaks as usual
/// (the call goes back into `break_block`), then the tool's postMine runs: only after a
/// survival break, as ServerPlayerInteractionManager.tryBreakBlock returns before postMine in
/// creative.
pub(crate) fn break_block(game: &mut Game<'_>, at: [i32; 3]) -> ControlFlow<Result<()>> {
    if BREAKING.get() {
        return ControlFlow::Continue(());
    }
    let Some(held) = game.held().cloned() else {
        return ControlFlow::Continue(());
    };
    let Some(kind) = kind(&held.item) else {
        return ControlFlow::Continue(());
    };
    if game.mode_id() != 0 || sneaking(game) {
        return ControlFlow::Continue(());
    }
    let before = state_at(game, at);
    let facts = game.res.facts().get(before).clone();
    if !takes(game, kind, &facts) {
        return ControlFlow::Continue(());
    }
    // LastBreakData.getFace: north when the block wasn't hit first.
    let face = FACES.with(|f| f.borrow_mut().remove(&at)).unwrap_or([0, 0, -1]);
    BREAKING.set(true);
    let result = game.break_block(at);
    if result.is_ok() && state_at(game, at) != before {
        match kind {
            Kind::LumberAxe if FALLING_TREES => {}
            Kind::LumberAxe => fell(game, at, &facts.name),
            // ScytheItem: always the horizontal 3x3.
            Kind::Scythe => around(game, at, [0, 1, 0], kind),
            _ => around(game, at, face, kind),
        }
    }
    BREAKING.set(false);
    ControlFlow::Break(result)
}

/// The eight blocks around, each broken as the player breaks a block
/// (ServerPlayerInteractionManager.tryBreakBlock) when the tool takes it.
fn around(game: &mut Game<'_>, at: [i32; 3], face: [i32; 3], kind: Kind) {
    for a in -1..=1 {
        for b in -1..=1 {
            if a == 0 && b == 0 {
                continue;
            }
            let p = offset(at, face, a, b);
            let facts = facts_at(game, p);
            if is_air(&facts.name) || facts.hardness < 0.0 || !takes(game, kind, &facts) {
                continue;
            }
            // HammerItem: only what the hammer in hand (if any is left) can harvest.
            if kind == Kind::Hammer
                && !game.res.items().correct_for_drops(game.held(), &facts.name)
            {
                continue;
            }
            let _ = game.break_block(p);
        }
    }
}

fn neighbours26(p: [i32; 3]) -> impl Iterator<Item = [i32; 3]> {
    (-1..=1).flat_map(move |dx| {
        (-1..=1).flat_map(move |dy| {
            (-1..=1).filter_map(move |dz| {
                (dx != 0 || dy != 0 || dz != 0).then_some([p[0] + dx, p[1] + dy, p[2] + dz])
            })
        })
    })
}

const SIDES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn add(p: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [p[0] + d[0], p[1] + d[1], p[2] + d[2]]
}

/// LumberAxeItem.isNaturalBlock.
fn natural(game: &Game<'_>, p: [i32; 3]) -> bool {
    let facts = facts_at(game, p);
    let name = &*facts.name;
    is_air(name)
        || facts.fluid_level > 0
        || facts.waterlogged
        || ["minecraft:logs", "minecraft:leaves", "minecraft:dirt", "minecraft:sand", "minecraft:snow",
            "minecraft:beehives", "minecraft:fire"]
            .iter()
            .any(|tag| game.block_in_tag(tag, name))
        || plant(&facts)
        || facts.replaceable
}

/// LumberAxeItem.detectTree: the logs of the broken one's kind joined to it at or above it,
/// none beside a block a player could have put there, at most 256, and at least three
/// non-persistent leaves by the top two layers.
fn detect(game: &Game<'_>, start: [i32; 3], log: &str) -> Vec<[i32; 3]> {
    let same = |p: [i32; 3]| &*facts_at(game, p).name == log;
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    for n in neighbours26(start) {
        if n[1] >= start[1] && same(n) {
            queue.push_back(n);
            visited.insert(n);
        }
    }
    visited.insert(start);
    let mut logs = Vec::new();
    while let Some(cur) = queue.pop_front() {
        if !same(cur) {
            continue;
        }
        if SIDES.iter().any(|d| !natural(game, add(cur, *d))) {
            return Vec::new();
        }
        logs.push(cur);
        if logs.len() > MAX_TREE {
            return Vec::new();
        }
        for n in neighbours26(cur) {
            if n[1] >= start[1] && visited.insert(n) && same(n) {
                queue.push_back(n);
            }
        }
    }
    let Some(top) = logs.iter().map(|p| p[1]).max() else {
        return Vec::new();
    };
    let reg = game.pack.assets.registry;
    let mut leaves = 0;
    for p in logs.iter().filter(|p| p[1] >= top - 1) {
        for d in SIDES {
            let q = add(*p, d);
            let id = state_at(game, q);
            let name = game.res.facts().get(id).name.clone();
            let persistent = reg
                .state(id)
                .and_then(|s| s.properties.get("persistent").cloned())
                .is_some_and(|v| v == "true");
            if game.block_in_tag("minecraft:leaves", &name) && !persistent {
                leaves += 1;
                if leaves >= MIN_LEAVES {
                    return logs;
                }
            }
        }
    }
    Vec::new()
}

/// LumberAxeItem.postMine: the tree's logs go (Level.removeBlock), their drops by the axe
/// fall at the broken log, and the axe takes one use per log. The leaves then decay on their
/// own (LeafDecayScheduler only hurried them).
fn fell(game: &mut Game<'_>, at: [i32; 3], log: &str) {
    let logs = detect(game, at, log);
    if logs.is_empty() {
        return;
    }
    let Some(air) = game.pack.assets.registry.default_state("minecraft:air") else {
        return;
    };
    let mut drops = Vec::new();
    for p in &logs {
        let Some(tool) = game.held().cloned() else {
            break;
        };
        let state = state_at(game, *p);
        let name = game.res.facts().get(state).name.clone();
        if !game.block_in_tag("minecraft:logs", &name) {
            continue;
        }
        let entity = game.loot_block_entity(*p, &name);
        drops.extend(game.block_loot_at(*p, state, Some(&tool), entity));
        let _ = game.level_set(*p, air);
    }
    for stack in drops {
        game.pop_resource(at, stack);
    }
    game.damage_held(logs.len() as u32);
}

/// ScytheItem.useOnBlock: a till that worked tills the eight around (each from above, with the
/// same scythe), unless sneaking.
pub(crate) fn till(game: &mut Game<'_>, stack: &ItemStack, pos: [i32; 3], face: [i32; 3]) -> ControlFlow<Result<bool>> {
    if TILLING.get() || kind(&stack.item) != Some(Kind::Scythe) || sneaking(game) {
        return ControlFlow::Continue(());
    }
    TILLING.set(true);
    let result = game.use_block_transformer(stack, pos, face);
    if matches!(result, Ok(true)) {
        'ring: for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let Some(now) = game.held().cloned() else {
                    break 'ring;
                };
                let _ = game.use_block_transformer(&now, [pos[0] + dx, pos[1], pos[2] + dz], [0, 1, 0]);
            }
        }
    }
    TILLING.set(false);
    ControlFlow::Break(result)
}

// ---- the client's follow-up clicks ----

/// The client doesn't know the toolshed's items: for it a click with one passes. So after the
/// server's use on a block acted, it still sends UseItem for the main hand and the off hand's
/// use on the block, which would switch the chisel's mode, use a bucket again on what is
/// behind or place the off hand's block. Java's client had the mod and answered SUCCESS (or
/// FAIL) for the main hand, so nothing more was sent. The follow-ups came up to two ticks after
/// the use; they are dropped until the next click's main-hand use on a block, or for
/// FOLLOW_UP_TICKS.
const FOLLOW_UP_TICKS: i64 = 10;

fn mark_used(game: &Game<'_>) {
    USED_ON_BLOCK.set(Some((game.lan_acting_uuid.unwrap_or(0), game.world.game_time as i64)));
}

fn just_used(game: &Game<'_>) -> bool {
    let player = game.lan_acting_uuid.unwrap_or(0);
    let now = game.world.game_time as i64;
    USED_ON_BLOCK.get().is_some_and(|(p, t)| p == player && (0..=FOLLOW_UP_TICKS).contains(&(now - t)))
}

// ---- copper buckets ----

const TWINS: [(&str, &str); 5] = [
    ("tokimistoolshed:copper_bucket", "minecraft:bucket"),
    ("tokimistoolshed:copper_water_bucket", "minecraft:water_bucket"),
    ("tokimistoolshed:copper_lava_bucket", "minecraft:lava_bucket"),
    ("tokimistoolshed:copper_powder_snow_bucket", "minecraft:powder_snow_bucket"),
    ("tokimistoolshed:copper_milk_bucket", "minecraft:milk_bucket"),
];

fn vanilla_twin(item: &str) -> Option<&'static str> {
    TWINS.iter().find(|(c, _)| *c == item).map(|(_, v)| *v)
}

fn copper_twin(item: &str) -> Option<&'static str> {
    TWINS.iter().find(|(_, v)| *v == item).map(|(c, _)| *c)
}

/// The held copper bucket as its vanilla twin, for the vanilla code that keys on the id.
fn swap_in(game: &mut Game<'_>, original: &ItemStack, vanilla: &str) -> ItemStack {
    let mut twin = original.clone();
    twin.item = std::sync::Arc::from(vanilla);
    twin.damage = 0;
    *game.held_mut() = Some(twin.clone());
    twin
}

/// Gives the copper bucket back after a vanilla use: filled or emptied as the vanilla one was,
/// with its wear, one more for a bucket that was emptied (CopperBucketItem.getEmptiedCopperStack,
/// CopperBucketBehaviors.damageBucketAndSwap). Worn through, it breaks. In creative the copper
/// bucket stays as it was.
fn settle(game: &mut Game<'_>, original: &ItemStack) {
    if game.creative {
        *game.held_mut() = Some(original.clone());
        return;
    }
    let Some(after) = game.held().cloned() else {
        return;
    };
    let Some(copper) = copper_twin(&after.item) else {
        return;
    };
    let mut stack = after;
    stack.item = std::sync::Arc::from(copper);
    stack.damage = original.damage;
    if &*original.item != COPPER_BUCKET && copper == COPPER_BUCKET {
        stack.damage += 1;
    }
    if stack.damage >= BUCKET_USES {
        *game.held_mut() = None;
        game.player_equipment_broke(horizon_world::player_inventory::EquipmentSlot::MainHand, &stack);
        return;
    }
    *game.held_mut() = Some(stack);
}

/// `Game::use_tool` with a copper bucket: the vanilla bucket's use (fluids, powder snow,
/// cauldrons), then the copper one back.
pub(crate) fn use_bucket(game: &mut Game<'_>, stack: &ItemStack, allow: bool) -> ControlFlow<Result<horizon_client::first_person::UseResult>> {
    if BUCKET.get() {
        return ControlFlow::Continue(());
    }
    let Some(vanilla) = vanilla_twin(&stack.item) else {
        return ControlFlow::Continue(());
    };
    let Some(original) = game.held().cloned().filter(|h| h.item == stack.item) else {
        return ControlFlow::Continue(());
    };
    if matches!(game.guest_use, Some(GuestUse::Item(_))) && just_used(game) {
        return ControlFlow::Break(Ok(horizon_client::first_person::UseResult::Pass));
    }
    let twin = swap_in(game, &original, vanilla);
    BUCKET.set(true);
    let result = game.use_tool(&twin, allow);
    BUCKET.set(false);
    settle(game, &original);
    if matches!(game.guest_use, Some(GuestUse::OnBlock(_)))
        && matches!(result, Ok(horizon_client::first_person::UseResult::Success { .. }))
    {
        mark_used(game);
    }
    ControlFlow::Break(result)
}

/// CowMilkMixin: an empty copper bucket milks a cow or mooshroom (not a goat).
pub(crate) fn milk(game: &mut Game<'_>, target: Option<InteractTarget>) -> ControlFlow<Result<bool>> {
    if BUCKET.get() {
        return ControlFlow::Continue(());
    }
    let Some(original) = game.held().cloned().filter(|h| &*h.item == COPPER_BUCKET) else {
        return ControlFlow::Continue(());
    };
    let index = match target {
        Some(t) => game.mobs.iter().position(|m| m.id as i64 == t.id as i64),
        None => game.mob_hit().map(|(i, _)| i),
    };
    let cow = index.is_some_and(|i| {
        matches!(
            game.mobs[i].species(),
            horizon_client::mobs::Species::Cow | horizon_client::mobs::Species::Mooshroom
        )
    });
    if !cow {
        return ControlFlow::Continue(());
    }
    swap_in(game, &original, "minecraft:bucket");
    BUCKET.set(true);
    let result = game.interact_mob(target);
    BUCKET.set(false);
    settle(game, &original);
    ControlFlow::Break(result)
}

/// `Game::finish_consuming` for the copper milk bucket: drunk as milk, then the bucket's wear.
pub(crate) fn finish_drinking(
    game: &mut Game<'_>,
    info: &horizon_client::items::ItemInfo,
    consumable: &horizon_world::consumable::Consumable<String, horizon_client::items::ConsumeEffect>,
    stack: &ItemStack,
) -> ControlFlow<()> {
    if BUCKET.get() || &*stack.item != COPPER_MILK {
        return ControlFlow::Continue(());
    }
    BUCKET.set(true);
    game.finish_consuming(info, consumable, stack);
    BUCKET.set(false);
    drink(game, stack);
    ControlFlow::Break(())
}

/// CopperMilkBucketItem.finishUsing: the copper bucket comes back with one more use of wear
/// (the item's use_remainder gives a fresh one).
fn drink(game: &mut Game<'_>, stack: &ItemStack) {
    if game.creative || &*stack.item != COPPER_MILK {
        return;
    }
    let damage = stack.damage + 1;
    let slot = game.held_mut();
    let Some(bucket) = slot.as_mut().filter(|b| &*b.item == COPPER_BUCKET && b.damage == 0) else {
        return;
    };
    if damage >= BUCKET_USES {
        let gone = bucket.clone();
        *slot = None;
        game.player_equipment_broke(horizon_world::player_inventory::EquipmentSlot::MainHand, &gone);
        return;
    }
    bucket.damage = damage;
}

/// ShearsLootMixin: block loot for clippers is rolled as for shears.
pub(crate) fn loot(
    game: &mut Game<'_>,
    pos: [i32; 3],
    state: BlockStateId,
    held: Option<&ItemStack>,
    entity: &Option<horizon_world::nbt::Tag>,
) -> ControlFlow<Vec<ItemStack>> {
    if LOOT.get() {
        return ControlFlow::Continue(());
    }
    let Some(clippers) = held.filter(|h| &*h.item == "tokimistoolshed:clippers") else {
        return ControlFlow::Continue(());
    };
    let mut shears = clippers.clone();
    shears.item = std::sync::Arc::from("minecraft:shears");
    shears.damage = 0;
    LOOT.set(true);
    let drops = game.block_loot_at(pos, state, Some(&shears), entity.clone());
    LOOT.set(false);
    ControlFlow::Break(drops)
}

// ---- trowel, chisel, abacus ----

fn is_block_item(game: &Game<'_>, stack: &ItemStack) -> bool {
    game.res.items().get(&stack.item).is_some_and(|i| i.block.is_some())
}

fn shuffle(game: &mut Game<'_>, v: &mut [usize]) {
    for i in (1..v.len()).rev() {
        let j = ((game.world.random.next_float() * (i + 1) as f32) as usize).min(i);
        v.swap(i, j);
    }
}

fn custom_int(stack: &ItemStack, key: &str) -> Option<i32> {
    match stack.data()?.other("minecraft:custom_data")?.get(key)? {
        horizon_world::nbt::Tag::Int(v) => Some(*v),
        _ => None,
    }
}

fn set_custom_ints(stack: &mut ItemStack, values: &[(&str, i32)]) {
    let mut compound = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(horizon_world::nbt::Tag::Compound(c)) => c.clone(),
        _ => Default::default(),
    };
    for (k, v) in values {
        compound.insert((*k).into(), horizon_world::nbt::Tag::Int(*v));
    }
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::Compound(compound)));
}

fn action_bar(game: &mut Game<'_>, line: &str) {
    if let Some(player) = game.lan_acting_uuid {
        game.native_action_bar(player, horizon_client::text::Text::literal(line));
    }
}

fn play_at_player(game: &mut Game<'_>, sound: &str, volume: f32) {
    let at = game.player.as_ref().map(|p| [p.position.x, p.position.y, p.position.z]);
    game.audio.play(sound, at, volume, 1.0);
}

/// `Game::use_hand` for the trowel, chisels and abacus in the main hand.
pub(crate) fn use_hand(game: &mut Game<'_>, block_hit: Option<&Hit>) -> ControlFlow<Result<bool>> {
    if HAND.get() {
        return ControlFlow::Continue(());
    }
    let main = game.hand == horizon_client::first_person::Hand::Main;
    if main && block_hit.is_some() {
        // A click starts with the main hand's use on the block: a new click.
        USED_ON_BLOCK.set(None);
    }
    if (!main || block_hit.is_none()) && just_used(game) {
        // A follow-up of a click a toolshed item already answered (see `mark_used`).
        return ControlFlow::Break(Ok(true));
    }
    if !main {
        return ControlFlow::Continue(());
    }
    let Some(held) = game.held().cloned() else {
        return ControlFlow::Continue(());
    };
    let result = match held.item.strip_prefix(NS) {
        Some("trowel") => trowel(game, block_hit),
        Some("abacus") => abacus(game, block_hit, held),
        Some("iron_chisel") => chisel(game, block_hit, held, "minecraft:iron_pickaxe"),
        Some("diamond_chisel") => chisel(game, block_hit, held, "minecraft:diamond_pickaxe"),
        _ => return ControlFlow::Continue(()),
    };
    if block_hit.is_some() && matches!(result, ControlFlow::Break(Ok(true))) {
        mark_used(game);
    }
    result
}

/// Uses the block in `slot` (a hotbar slot, or the off hand for None) at `hit` as the player
/// would, the selected slot or hand put back after.
fn place_from(game: &mut Game<'_>, slot: Option<usize>, hit: &Hit) -> Result<bool> {
    let (selected, hand) = (game.selected, game.hand);
    match slot {
        Some(s) => game.selected = s,
        None => game.hand = horizon_client::first_person::Hand::Off,
    }
    HAND.set(true);
    let result = game.use_hand(Some(hit));
    HAND.set(false);
    game.selected = selected;
    game.hand = hand;
    result
}

/// TrowelItem.useOnBlock: a random block of the hotbar, placed as if it were held; the next
/// one when it can't go there.
fn trowel(game: &mut Game<'_>, block_hit: Option<&Hit>) -> ControlFlow<Result<bool>> {
    let Some(hit) = block_hit else {
        return ControlFlow::Continue(());
    };
    let mut slots: Vec<usize> = (0..9)
        .filter(|&i| game.inventory[i].as_ref().is_some_and(|s| is_block_item(game, s)))
        .collect();
    if slots.is_empty() {
        return ControlFlow::Continue(());
    }
    shuffle(game, &mut slots);
    for slot in slots {
        match place_from(game, Some(slot), hit) {
            Ok(false) => continue,
            other => return ControlFlow::Break(other),
        }
    }
    ControlFlow::Break(Ok(false))
}

/// AbacusItem.useOnBlock: binds to the block, or unbinds when bound.
fn abacus(game: &mut Game<'_>, block_hit: Option<&Hit>, mut held: ItemStack) -> ControlFlow<Result<bool>> {
    let Some(hit) = block_hit else {
        return ControlFlow::Continue(());
    };
    let line = if custom_int(&held, "boundPosY").is_some_and(|y| y != -999) {
        set_custom_ints(&mut held, &[("boundPosY", -999)]);
        "§6Abacus unbound".to_owned()
    } else {
        let p = hit.pos;
        set_custom_ints(&mut held, &[("boundPosX", p[0]), ("boundPosY", p[1]), ("boundPosZ", p[2])]);
        format!("§6Abacus bound to {}, {}, {}", p[0], p[1], p[2])
    };
    *game.held_mut() = Some(held);
    action_bar(game, &line);
    ControlFlow::Break(Ok(true))
}

/// AbacusHudRenderer, as a line over the hotbar: while a bound abacus is in either hand and a
/// block is aimed at, the blocks counted along each axis that differs (at most 128).
pub(crate) fn abacus_hud(game: &mut Game<'_>) {
    let tick = HUD_TICK.get().wrapping_add(1);
    HUD_TICK.set(tick);
    if tick % 5 != 0 {
        return;
    }
    let abacus = [game.inventory[game.selected].as_ref(), game.offhand.as_ref()]
        .into_iter()
        .flatten()
        .find(|s| &*s.item == "tokimistoolshed:abacus")
        .cloned();
    let Some(abacus) = abacus else { return };
    let (Some(x), Some(y), Some(z)) = (
        custom_int(&abacus, "boundPosX"),
        custom_int(&abacus, "boundPosY"),
        custom_int(&abacus, "boundPosZ"),
    ) else {
        return;
    };
    if y == -999 {
        return;
    }
    let Some(hit) = game.pick() else { return };
    let d = [hit.pos[0] - x, hit.pos[1] - y, hit.pos[2] - z].map(|v| v.abs().min(MAX_AXIS));
    let parts: Vec<String> = ["X", "Y", "Z"]
        .iter()
        .zip(d)
        .filter(|(_, v)| *v > 0)
        .map(|(axis, v)| format!("{axis} {}", v + 1))
        .collect();
    let line = if parts.is_empty() { "1".to_owned() } else { parts.join("   ") };
    action_bar(game, &line);
}

/// ChiselItem.useOnBlock: the block is swapped for one of the player's (the off hand's block in
/// Copy mode, a random hotbar block in Shuffle mode). The old block's drops, as the tier's
/// pickaxe with the chisel's enchantments would get them, go to the inventory, and the chisel
/// takes a use (it stops at its last one). Using it in the air switches the mode (on Java:
/// Left Alt and the mouse wheel, a client keybind).
fn chisel(game: &mut Game<'_>, block_hit: Option<&Hit>, mut held: ItemStack, pickaxe: &str) -> ControlFlow<Result<bool>> {
    let mode = custom_int(&held, "ChiselMode").unwrap_or(0).clamp(0, 1);
    let Some(hit) = block_hit else {
        let next = 1 - mode;
        set_custom_ints(&mut held, &[("ChiselMode", next)]);
        *game.held_mut() = Some(held);
        action_bar(game, if next == 0 { "Chisel mode: Copy" } else { "Chisel mode: Shuffle" });
        return ControlFlow::Break(Ok(true));
    };
    let pos = hit.pos;
    let state = state_at(game, pos);
    let facts = game.res.facts().get(state).clone();
    if is_air(&facts.name) {
        return ControlFlow::Continue(());
    }
    let max = game.res.items().get(&held.item).map_or(0, |i| i.max_damage);
    let mut tool = ItemStack::new(pickaxe, 1);
    if let Some(data) = held.data() {
        tool.data_mut().enchantments = data.enchantments.clone();
    }
    let worn = max > 0 && held.damage + 1 >= max;
    if worn || (facts.requires_tool && !game.res.items().correct_for_drops(Some(&tool), &facts.name)) {
        play_at_player(game, "minecraft:block.anvil.land", 0.8);
        return ControlFlow::Break(Ok(true));
    }
    let block_of = |game: &Game<'_>, s: &ItemStack| {
        game.res.items().get(&s.item).and_then(|i| i.block).map(|b| game.res.facts().get(b).name.clone())
    };
    let mut candidates: Vec<Option<usize>> = if mode == 0 {
        game.offhand.as_ref().filter(|s| is_block_item(game, s)).map(|_| vec![None]).unwrap_or_default()
    } else {
        let mut slots: Vec<usize> = (0..9)
            .filter(|&i| {
                game.inventory[i]
                    .as_ref()
                    .is_some_and(|s| is_block_item(game, s) && !kind_is_chisel(&s.item))
            })
            .collect();
        shuffle(game, &mut slots);
        slots.into_iter().map(Some).collect()
    };
    candidates.retain(|slot| {
        let stack = match slot {
            Some(i) => game.inventory[*i].as_ref(),
            None => game.offhand.as_ref(),
        };
        stack.and_then(|s| block_of(game, s)).is_some_and(|b| b != facts.name)
    });
    if candidates.is_empty() {
        return ControlFlow::Continue(());
    }
    let Some(air) = game.pack.assets.registry.default_state("minecraft:air") else {
        return ControlFlow::Continue(());
    };
    for slot in candidates {
        let entity = game.loot_block_entity(pos, &facts.name);
        let drops = game.block_loot_at(pos, state, Some(&tool), entity);
        if game.level_set(pos, air).is_err() {
            break;
        }
        let placed = place_from(game, slot, hit);
        if placed.is_ok() && state_at(game, pos) != air {
            if !game.creative {
                if let Some(chisel) = game.held_mut().as_mut() {
                    chisel.damage = (chisel.damage + 1).min(max.saturating_sub(1));
                }
                for drop in drops {
                    if let Some(rest) = game.give(drop) {
                        game.player_drop(rest);
                    }
                }
            }
            return ControlFlow::Break(Ok(true));
        }
        let _ = game.level_set(pos, state);
        if placed.is_err() {
            return ControlFlow::Break(placed);
        }
    }
    ControlFlow::Break(Ok(false))
}

fn kind_is_chisel(item: &str) -> bool {
    item.ends_with("_chisel") && item.starts_with(NS)
}

