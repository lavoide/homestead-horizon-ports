// Farmer's Delight 2.4.1+refabricated (vectorwing, MehVahdJukaar, MerchantPug; MIT) for Horizon:
// the Java half of the content mod mods/farmersdelight. Ported from the decompiled
// `vectorwing.farmersdelight.common` classes, with Homestead's farmersdelight-common.toml
// (richSoilBoostChance 0.8, cuttingBoardFortuneBonus 0.1).
//
// - block entities: the stove, cooking pot, skillet, cutting board, basket and cabinets, with
//   their ticks, slots, the items drawn on them and the cooking pot's screen;
// - right clicks: lighting and putting out the stove, serving from the pot and its handle,
//   the skillet, the cutting board (cutting recipes, carving), pies, feasts, tomato picking,
//   shearing colonies; planting seed items, tilling rich soil, extending rope;
// - the level: rich soil, rich farmland, compost, tomatoes, rice and colonies grow as in Java,
//   bone meal on them, where they survive, and the states blocks are placed in;
// - foods: the drinks' own effects, Comfort and Nourishment, and the lit stove's burn.
//
// Injected as `crate::game::farmersdelight`.
use super::*;
use horizon_client::containers::{ModBlockEntity, ModContainer};
use horizon_client::effects::EffectAction;
use horizon_world::nbt::{Compound, JavaString, Tag};

const NS: &str = "farmersdelight:";
const RICH_SOIL_BOOST: f32 = 0.8;
const FORTUNE_BONUS: f64 = 0.1;

// ---------------------------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------------------------

fn short(name: &str) -> Option<&str> {
    name.strip_prefix(NS)
}

/// Direction by name, as (x, z) steps; `None` for up and down.
fn step(dir: &str) -> (i32, i32) {
    match dir {
        "north" => (0, -1),
        "south" => (0, 1),
        "west" => (-1, 0),
        "east" => (1, 0),
        _ => (0, 0),
    }
}
fn opposite(dir: &str) -> &'static str {
    match dir {
        "north" => "south",
        "south" => "north",
        "west" => "east",
        "east" => "west",
        "up" => "down",
        _ => "up",
    }
}
fn clockwise(dir: &str) -> &'static str {
    match dir {
        "north" => "east",
        "east" => "south",
        "south" => "west",
        _ => "north",
    }
}
fn counter_clockwise(dir: &str) -> &'static str {
    match dir {
        "north" => "west",
        "west" => "south",
        "south" => "east",
        _ => "north",
    }
}
/// `Direction.toYRot`.
fn y_rot(dir: &str) -> f32 {
    match dir {
        "south" => 0.0,
        "west" => 90.0,
        "north" => 180.0,
        _ => 270.0,
    }
}
/// `Direction.fromYRot`: the placer's horizontal facing.
fn horizontal(yaw: f32) -> &'static str {
    match ((yaw / 90.0 + 0.5).floor() as i32) & 3 {
        0 => "south",
        1 => "west",
        2 => "north",
        _ => "east",
    }
}
fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

fn key(s: &str) -> JavaString {
    JavaString::from(s)
}
fn nbt_int(nbt: &Compound, k: &str) -> i32 {
    match nbt.get(&key(k)) {
        Some(Tag::Int(v)) => *v,
        Some(Tag::Short(v)) => i32::from(*v),
        Some(Tag::Byte(v)) => i32::from(*v),
        _ => 0,
    }
}
fn nbt_long(nbt: &Compound, k: &str) -> i64 {
    match nbt.get(&key(k)) {
        Some(Tag::Long(v)) => *v,
        Some(Tag::Int(v)) => i64::from(*v),
        _ => 0,
    }
}
fn nbt_ints(nbt: &Compound, k: &str, n: usize) -> Vec<i32> {
    let mut out = match nbt.get(&key(k)) {
        Some(Tag::IntArray(v)) => v.clone(),
        _ => Vec::new(),
    };
    out.resize(n, 0);
    out
}
fn nbt_str(nbt: &Compound, k: &str) -> Option<String> {
    match nbt.get(&key(k)) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()).filter(|s| !s.is_empty()),
        _ => None,
    }
}
fn put(nbt: &mut Compound, k: &str, v: Tag) {
    if nbt.get(&key(k)) != Some(&v) {
        nbt.insert(key(k), v);
    }
}
fn string_tag(s: &str) -> Tag {
    Tag::String(JavaString::from(s))
}

fn sized(items: &mut Vec<Option<ItemStack>>, n: usize) {
    if items.len() < n {
        items.resize(n, None);
    }
}
fn stack_of(id: &str, count: u32) -> ItemStack {
    ItemStack::new(id, count)
}

// ---------------------------------------------------------------------------------------------
// Block entities
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Stove,
    Pot,
    Skillet,
    Board,
    Basket,
    Cabinet,
}

pub(crate) fn kind_of_block(block: &str) -> Option<Kind> {
    let n = short(block)?;
    Some(match n {
        "stove" => Kind::Stove,
        "cooking_pot" => Kind::Pot,
        "skillet" => Kind::Skillet,
        "cutting_board" => Kind::Board,
        "basket" => Kind::Basket,
        _ if n.ends_with("_cabinet") => Kind::Cabinet,
        _ => return None,
    })
}
fn entity_id(k: Kind) -> &'static str {
    match k {
        Kind::Stove => "farmersdelight:stove",
        Kind::Pot => "farmersdelight:cooking_pot",
        Kind::Skillet => "farmersdelight:skillet",
        Kind::Board => "farmersdelight:cutting_board",
        Kind::Basket => "farmersdelight:basket",
        Kind::Cabinet => "farmersdelight:cabinet",
    }
}
fn kind_of_entity(id: &str) -> Option<Kind> {
    Some(match id {
        "farmersdelight:stove" => Kind::Stove,
        "farmersdelight:cooking_pot" => Kind::Pot,
        "farmersdelight:skillet" => Kind::Skillet,
        "farmersdelight:cutting_board" => Kind::Board,
        "farmersdelight:basket" => Kind::Basket,
        "farmersdelight:cabinet" => Kind::Cabinet,
        _ => return None,
    })
}

/// `EntityBlock.newBlockEntity`.
pub(crate) fn block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if entity.is_none()
        && let Some(k) = kind_of_block(block)
    {
        *entity = Some((entity_id(k).to_owned(), Compound::new()));
    }
}

/// `EntityBlock.getTicker`.
pub(crate) fn ticks(block: &str) -> bool {
    matches!(kind_of_block(block), Some(Kind::Stove | Kind::Pot | Kind::Skillet | Kind::Basket))
}

pub(crate) const POT_MENU: &str = "farmersdelight:cooking_pot";
const MEAL: usize = 6;
const CONTAINER: usize = 7;
const OUTPUT: usize = 8;

/// The block entities' item slots.
pub(crate) fn container(block: &str) -> Option<ModContainer> {
    let k = kind_of_block(block)?;
    let all = |n: usize| -> ModContainer {
        let s: Vec<usize> = (0..n).collect();
        ModContainer {
            slots: n,
            faces: std::array::from_fn(|_| s.clone()),
            inputs: s.clone(),
            outputs: s,
            menu: Some("minecraft:generic_9x3".into()),
        }
    };
    Some(match k {
        Kind::Stove => ModContainer { slots: 6, ..Default::default() },
        Kind::Skillet | Kind::Board => ModContainer { slots: 1, ..Default::default() },
        // CookingPotItemHandler: the top reaches the ingredients, the other faces the container
        // slot and the output.
        Kind::Pot => {
            let inputs: Vec<usize> = (0..6).collect();
            let side = vec![CONTAINER, OUTPUT];
            ModContainer {
                slots: 9,
                faces: [
                    side.clone(),
                    inputs.clone(),
                    side.clone(),
                    side.clone(),
                    side.clone(),
                    side,
                ],
                inputs: (0..6).chain([CONTAINER]).collect(),
                outputs: vec![OUTPUT],
                menu: Some(POT_MENU.into()),
            }
        }
        Kind::Basket | Kind::Cabinet => all(27),
    })
}

/// What clients see of a block entity: the items drawn on it.
pub(crate) fn update_tag(entity: &ModBlockEntity, tag: &mut Option<Compound>) {
    let Some(k) = kind_of_entity(&entity.id) else { return };
    if !matches!(k, Kind::Stove | Kind::Skillet | Kind::Board) {
        return;
    }
    let mut c = Compound::new();
    c.insert(key("Items"), horizon_items::item_nbt::encode_items(&entity.items));
    if k == Kind::Board {
        c.insert(key("IsItemCarved"), Tag::Byte(nbt_int(&entity.nbt, "IsItemCarved") as i8));
    }
    if k == Kind::Skillet {
        c.insert(key("FlipTime"), Tag::Long(nbt_long(&entity.nbt, "FlipTime")));
    }
    *tag = Some(c);
}

fn client_items(entity: &ModBlockEntity, n: usize) -> Vec<Option<ItemStack>> {
    horizon_items::item_nbt::decode_items(entity.nbt.get(&key("Items")), n)
}

// ---------------------------------------------------------------------------------------------
// Reading the world from the game
// ---------------------------------------------------------------------------------------------

fn state_at(game: &Game<'_>, p: [i32; 3]) -> BlockStateId {
    game.world.block(p[0], p[1], p[2])
}
fn name_at(game: &Game<'_>, p: [i32; 3]) -> Arc<str> {
    game.res.facts().get(state_at(game, p)).name.clone()
}
fn prop(game: &Game<'_>, p: [i32; 3], k: &str) -> Option<String> {
    game.pack.assets.registry.state(state_at(game, p))?.properties.get(k).cloned()
}
/// The state at `p` with `changes` applied (its block unchanged).
fn with_props(game: &Game<'_>, p: [i32; 3], changes: &[(&str, &str)]) -> Option<BlockStateId> {
    let reg = game.pack.assets.registry;
    let mut st = reg.state(state_at(game, p))?.clone();
    for (k, v) in changes {
        st.properties.insert((*k).into(), (*v).into());
    }
    reg.resolve(&st).ok()
}
fn set_props(game: &mut Game<'_>, p: [i32; 3], changes: &[(&str, &str)]) -> bool {
    match with_props(game, p, changes) {
        Some(s) => game.level_set(p, s).is_ok(),
        None => false,
    }
}
fn default_of(game: &Game<'_>, name: &str) -> Option<BlockStateId> {
    game.pack.assets.registry.default_state(name)
}
fn set_default(game: &mut Game<'_>, p: [i32; 3], name: &str, changes: &[(&str, &str)]) -> bool {
    let reg = game.pack.assets.registry;
    let Some(mut st) = default_of(game, name).and_then(|d| reg.state(d)).cloned() else { return false };
    for (k, v) in changes {
        st.properties.insert((*k).into(), (*v).into());
    }
    match reg.resolve(&st) {
        Ok(s) => game.level_set(p, s).is_ok(),
        Err(_) => false,
    }
}
fn remove_block(game: &mut Game<'_>, p: [i32; 3]) {
    if let Some(air) = default_of(game, "minecraft:air") {
        let _ = game.level_set(p, air);
    }
}
fn block_tag(game: &Game<'_>, tag: &str, block: &str) -> bool {
    game.items().block_in_tag(tag, block)
}
fn item_tag(game: &Game<'_>, tag: &str, item: &str) -> bool {
    game.items().item_in_tag(tag, item)
}
fn is_knife(game: &Game<'_>, item: &str) -> bool {
    item_tag(game, "c:tools/knives", item) || item_tag(game, "farmersdelight:tools/knives", item)
}
fn is_shears(game: &Game<'_>, item: &str) -> bool {
    item == "minecraft:shears" || item_tag(game, "c:shears", item) || item_tag(game, "c:tools/shears", item)
}
fn max_stack(game: &Game<'_>, item: &str) -> u32 {
    game.items().get(item).map_or(64, |i| i.max_stack)
}
fn remainder(game: &Game<'_>, item: &str) -> Option<String> {
    game.items().get(item).and_then(|i| i.crafting_remainder.clone())
}
fn sneaking(game: &Game<'_>) -> bool {
    game.keys.contains(&KeyCode::ShiftLeft)
}
fn centre(p: [i32; 3]) -> [f64; 3] {
    p.map(|c| f64::from(c) + 0.5)
}
fn play(game: &mut Game<'_>, sound: &str, at: [f64; 3], volume: f32, pitch: f32) {
    game.audio.play(sound, Some(at), volume, pitch);
}
fn tell(game: &mut Game<'_>, text: horizon_client::text::Text) {
    if let Some(player) = game.native_acting_player() {
        game.native_action_bar(player.uuid, text);
    }
}
/// The held hand's stack shrinks by one (none in creative).
fn use_one(game: &mut Game<'_>) {
    game.consume_held();
}
/// Into the player's inventory, else dropped at their feet.
fn give_or_drop(game: &mut Game<'_>, stack: ItemStack) {
    if let Some(rest) = game.give(stack) {
        game.player_drop(rest);
    }
}
/// `ItemUtils.spawnItemEntity`.
fn spawn(game: &mut Game<'_>, stack: ItemStack, at: [f64; 3], vel: [f64; 3]) {
    game.spawn_item(stack, at, vel, 10);
}
/// `Block.popResource`: a random spot in the block.
fn pop(game: &mut Game<'_>, p: [i32; 3], stack: ItemStack) {
    let r = &mut game.world.random;
    let at = [
        f64::from(p[0]) + 0.5 + (r.next_double() - 0.5) * 0.5,
        f64::from(p[1]) + 0.5 + (r.next_double() - 0.5) * 0.5 - 0.125,
        f64::from(p[2]) + 0.5 + (r.next_double() - 0.5) * 0.5,
    ];
    let vel = [r.next_double() * 0.2 - 0.1, 0.2, r.next_double() * 0.2 - 0.1];
    game.spawn_item(stack, at, vel, 10);
}

/// `HeatableBlockEntity.isHeated`: a heat source below (lit, if it has `lit`), or one under a
/// heat conductor.
fn heated(game: &Game<'_>, p: [i32; 3], direct: bool) -> bool {
    let source = |game: &Game<'_>, q: [i32; 3]| -> Option<bool> {
        let name = name_at(game, q);
        block_tag(game, "farmersdelight:heat_sources", &name)
            .then(|| prop(game, q, "lit").is_none_or(|v| v == "true"))
    };
    let below = [p[0], p[1] - 1, p[2]];
    if let Some(lit) = source(game, below) {
        return lit;
    }
    if !direct && block_tag(game, "farmersdelight:heat_conductors", &name_at(game, below)) {
        return source(game, [p[0], p[1] - 2, p[2]]).unwrap_or(false);
    }
    false
}
fn tray_state(game: &Game<'_>, p: [i32; 3]) -> bool {
    block_tag(game, "farmersdelight:tray_heat_sources", &name_at(game, [p[0], p[1] - 1, p[2]]))
}

/// `StoveBlockEntity.isStoveBlockedAbove`: the block above has a shape in the grill area
/// (3..13 × 0..1 × 3..13 pixels of it).
fn stove_blocked(game: &Game<'_>, p: [i32; 3]) -> bool {
    let facts = game.res.facts().get(state_at(game, [p[0], p[1] + 1, p[2]]));
    let (lo, hi) = (3.0 / 16.0, 13.0 / 16.0);
    facts.outline.iter().any(|b| b[0] < hi && b[3] > lo && b[1] < 1.0 / 16.0 && b[4] > 0.0 && b[2] < hi && b[5] > lo)
}

fn campfire(game: &Game<'_>, stack: &ItemStack) -> Option<(ItemStack, u32)> {
    game.recipes().cook("minecraft:campfire_cooking", stack).map(|c| (c.result.clone(), c.time))
}

// ---------------------------------------------------------------------------------------------
// Ticks
// ---------------------------------------------------------------------------------------------

pub(crate) fn tick(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let Some(k) = kind_of_entity(&entity.id) else { return };
    match k {
        Kind::Stove => stove_tick(game, pos, entity),
        Kind::Pot => pot_tick(game, pos, entity),
        Kind::Skillet => skillet_tick(game, pos, entity),
        Kind::Basket => basket_tick(game, pos, entity),
        _ => {}
    }
}

/// `StoveBlockEntity.cookingTick`.
fn stove_tick(game: &mut Game<'_>, pos: [i32; 3], e: &mut ModBlockEntity) {
    let lit = prop(game, pos, "lit").is_some_and(|v| v == "true");
    let mut times = nbt_ints(&e.nbt, "CookingTimes", 6);
    let totals = nbt_ints(&e.nbt, "CookingTotalTimes", 6);
    let mut changed = false;
    if stove_blocked(game, pos) {
        if e.items.iter().any(Option::is_some) {
            let items = std::mem::take(&mut e.items);
            for s in items.into_iter().flatten() {
                pop(game, pos, s);
            }
            sized(&mut e.items, 6);
            changed = true;
        }
    } else if lit {
        sized(&mut e.items, 6);
        for i in 0..6 {
            let Some(s) = e.items[i].clone() else { continue };
            times[i] += 1;
            if times[i] >= totals[i] {
                if let Some((result, _)) = campfire(game, &s) {
                    let r = &mut game.world.random;
                    let vel = [r.next_gaussian() * 0.01, 0.1, r.next_gaussian() * 0.01];
                    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 1.0, f64::from(pos[2]) + 0.5];
                    spawn(game, result, at, vel);
                }
                e.items[i] = None;
                changed = true;
            }
        }
    } else {
        for i in 0..6 {
            if times[i] > 0 {
                times[i] = (times[i] - 2).clamp(0, totals[i].max(0));
            }
        }
    }
    if nbt_ints(&e.nbt, "CookingTimes", 6) != times {
        put(&mut e.nbt, "CookingTimes", Tag::IntArray(times));
    }
    if changed {
        game.native_block_entity_sync(pos);
    }
}

/// `CookingPotRecipe`, as its file says.
#[derive(Clone)]
struct PotRecipe {
    id: String,
    ingredients: Vec<horizon_client::crafting::Ingredient>,
    result: ItemStack,
    container: Option<String>,
    time: i32,
}

thread_local! {
    static POT_RECIPES: std::cell::RefCell<(usize, usize, Arc<Vec<PotRecipe>>)> =
        std::cell::RefCell::new((0, 0, Arc::new(Vec::new())));
}

/// The cooking recipes, read again whenever the recipe table changes.
fn pot_recipes(game: &Game<'_>) -> Arc<Vec<PotRecipe>> {
    let list = game.native_recipes_of_type("farmersdelight:cooking");
    let tag = (list.as_ptr() as usize, list.len());
    if let Some(r) = POT_RECIPES.with(|c| {
        let c = c.borrow();
        ((c.0, c.1) == tag).then(|| Arc::clone(&c.2))
    }) {
        return r;
    }
    let mut out = Vec::new();
    for r in list {
        let j = &r.json;
        let Some(result) = horizon_client::crafting::recipe_result(&j["result"]) else { continue };
        let ingredients: Option<Vec<_>> = j["ingredients"]
            .as_array()
            .map(|a| a.iter().map(|i| game.native_recipe_ingredient(i)).collect::<Option<Vec<_>>>())
            .unwrap_or(None);
        let Some(ingredients) = ingredients else { continue };
        let container = j["container"]["id"]
            .as_str()
            .or(j["container"]["item"].as_str())
            .map(horizon_client::atlas_sources::qualified)
            .or_else(|| remainder(game, &result.item));
        out.push(PotRecipe {
            id: r.id.clone(),
            ingredients,
            result,
            container,
            time: j["cookingtime"].as_i64().unwrap_or(200) as i32,
        });
    }
    let out = Arc::new(out);
    POT_RECIPES.with(|c| *c.borrow_mut() = (tag.0, tag.1, Arc::clone(&out)));
    out
}

/// `RecipeMatcher.findMatches`: each filled input slot to its own ingredient.
fn shapeless(ingredients: &[horizon_client::crafting::Ingredient], stacks: &[&ItemStack]) -> bool {
    if ingredients.len() != stacks.len() {
        return false;
    }
    fn place(
        i: usize,
        ingredients: &[horizon_client::crafting::Ingredient],
        stacks: &[&ItemStack],
        used: &mut [bool],
    ) -> bool {
        if i == ingredients.len() {
            return true;
        }
        for (s, stack) in stacks.iter().enumerate() {
            if !used[s] && ingredients[i].test(stack) {
                used[s] = true;
                if place(i + 1, ingredients, stacks, used) {
                    return true;
                }
                used[s] = false;
            }
        }
        false
    }
    place(0, ingredients, stacks, &mut vec![false; stacks.len()])
}

/// `CookingPotBlockEntity.INGREDIENT_REMAINDER_OVERRIDES`.
fn remainder_override(item: &str) -> Option<&'static str> {
    Some(match item {
        "minecraft:powder_snow_bucket"
        | "minecraft:axolotl_bucket"
        | "minecraft:cod_bucket"
        | "minecraft:pufferfish_bucket"
        | "minecraft:salmon_bucket"
        | "minecraft:tropical_fish_bucket" => "minecraft:bucket",
        "minecraft:suspicious_stew" | "minecraft:mushroom_stew" | "minecraft:rabbit_stew" | "minecraft:beetroot_soup" => {
            "minecraft:bowl"
        }
        "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion" | "minecraft:experience_bottle" => {
            "minecraft:glass_bottle"
        }
        _ => return None,
    })
}

/// `CookingPotBlockEntity.cookingTick`.
fn pot_tick(game: &mut Game<'_>, pos: [i32; 3], e: &mut ModBlockEntity) {
    sized(&mut e.items, 9);
    let is_heated = heated(game, pos, false);
    let mut cook = nbt_int(&e.nbt, "CookTime");
    let mut total = nbt_int(&e.nbt, "CookTimeTotal");
    let has_input = e.items[..6].iter().any(Option::is_some);
    let mut changed = false;
    if is_heated && has_input {
        let recipes = pot_recipes(game);
        let filled: Vec<&ItemStack> = e.items[..6].iter().flatten().collect();
        let found = recipes.iter().find(|r| shapeless(&r.ingredients, &filled)).cloned();
        drop(filled);
        let can_cook = found.as_ref().is_some_and(|r| match &e.items[MEAL] {
            None => true,
            Some(m) => m.item == r.result.item && m.count + r.result.count <= max_stack(game, &r.result.item),
        });
        match found.filter(|_| can_cook) {
            Some(r) => {
                if nbt_str(&e.nbt, "LastRecipe").as_deref() != Some(&r.id) {
                    if nbt_str(&e.nbt, "LastRecipe").is_some() {
                        cook = 0;
                    }
                    put(&mut e.nbt, "LastRecipe", string_tag(&r.id));
                }
                cook += 1;
                total = r.time;
                if cook >= total {
                    cook = 0;
                    match &r.container {
                        Some(c) => put(&mut e.nbt, "Container", string_tag(c)),
                        None => {
                            e.nbt.remove(&key("Container"));
                        }
                    }
                    match &mut e.items[MEAL] {
                        Some(m) => m.count += r.result.count,
                        slot => *slot = Some(r.result.clone()),
                    }
                    let facing = prop(game, pos, "facing").unwrap_or_else(|| "north".into());
                    let (dx, dz) = step(counter_clockwise(&facing));
                    for i in 0..6 {
                        let Some(s) = e.items[i].clone() else { continue };
                        let rest = remainder(game, &s.item).or_else(|| remainder_override(&s.item).map(String::from));
                        if let Some(rest) = rest {
                            let at = [
                                f64::from(pos[0]) + 0.5 + f64::from(dx) * 0.25,
                                f64::from(pos[1]) + 0.7,
                                f64::from(pos[2]) + 0.5 + f64::from(dz) * 0.25,
                            ];
                            spawn(game, stack_of(&rest, 1), at, [f64::from(dx) * 0.08, 0.25, f64::from(dz) * 0.08]);
                        }
                        let slot = &mut e.items[i];
                        if let Some(st) = slot {
                            st.count -= 1;
                            if st.count == 0 {
                                *slot = None;
                            }
                        }
                    }
                    changed = true;
                }
            }
            None => cook = (cook - 2).clamp(0, total.max(0)),
        }
    } else if cook > 0 {
        cook = (cook - 2).clamp(0, total.max(0));
    }

    if let Some(meal) = e.items[MEAL].clone() {
        let container = nbt_str(&e.nbt, "Container").or_else(|| remainder(game, &meal.item));
        let max = max_stack(game, &meal.item);
        match container {
            // moveMealToOutput
            None => {
                let out = e.items[OUTPUT].clone();
                if out.as_ref().is_none_or(|o| o.item == meal.item) {
                    let have = out.as_ref().map_or(0, |o| o.count);
                    let n = meal.count.min(max.saturating_sub(have));
                    if n > 0 {
                        e.items[OUTPUT] = Some(ItemStack { count: have + n, ..meal.clone() });
                        e.items[MEAL] = (meal.count > n).then(|| ItemStack { count: meal.count - n, ..meal.clone() });
                        changed = true;
                    }
                }
            }
            // useStoredContainersOnMeal
            Some(c) => {
                if let Some(held) = e.items[CONTAINER].clone().filter(|h| *h.item == *c) {
                    let out = e.items[OUTPUT].clone();
                    let have = out.as_ref().map_or(0, |o| o.count);
                    if out.as_ref().is_none_or(|o| o.item == meal.item) && have < max {
                        let n = meal.count.min(held.count).min(max - have);
                        if n > 0 {
                            e.items[OUTPUT] = Some(ItemStack { count: have + n, ..meal.clone() });
                            e.items[MEAL] = (meal.count > n).then(|| ItemStack { count: meal.count - n, ..meal.clone() });
                            e.items[CONTAINER] = (held.count > n).then(|| ItemStack { count: held.count - n, ..held });
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    put(&mut e.nbt, "CookTime", Tag::Int(cook));
    put(&mut e.nbt, "CookTimeTotal", Tag::Int(total));
    let data = vec![cook, total, i32::from(is_heated)];
    if e.data != data {
        e.data = data;
    }
    let _ = changed;
}

/// `SkilletBlock.getSkilletCookingTime`.
fn skillet_time(original: u32) -> i32 {
    let original = if original > 0 { original as i32 } else { 600 };
    let result = ((original / 20) as f32 * 0.2) as i32 * 20;
    result.clamp(60.min(original), original)
}

/// `SkilletBlockEntity.cookingTick`.
fn skillet_tick(game: &mut Game<'_>, pos: [i32; 3], e: &mut ModBlockEntity) {
    sized(&mut e.items, 1);
    let mut cook = nbt_int(&e.nbt, "CookTime");
    let total = nbt_int(&e.nbt, "CookTimeTotal");
    let mut changed = false;
    if heated(game, pos, true) {
        match e.items[0].clone() {
            None => cook = 0,
            Some(s) => {
                cook += 1;
                if cook >= total
                    && let Some((result, _)) = campfire(game, &s)
                {
                    let facing = prop(game, pos, "facing").unwrap_or_else(|| "north".into());
                    let (dx, dz) = step(clockwise(&facing));
                    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.3, f64::from(pos[2]) + 0.5];
                    spawn(game, result, at, [f64::from(dx) * 0.08, 0.25, f64::from(dz) * 0.08]);
                    cook = 0;
                    e.items[0] = (s.count > 1).then(|| ItemStack { count: s.count - 1, ..s });
                    changed = true;
                }
            }
        }
    } else if cook > 0 {
        cook = (cook - 2).clamp(0, total.max(0));
    }
    put(&mut e.nbt, "CookTime", Tag::Int(cook));
    if changed {
        game.native_block_entity_sync(pos);
    }
}

/// `BasketBlockEntity.pushItemsTick`: items in front of its opening go in, every 8 ticks at most.
fn basket_tick(game: &mut Game<'_>, pos: [i32; 3], e: &mut ModBlockEntity) {
    sized(&mut e.items, 27);
    let cooldown = nbt_int(&e.nbt, "TransferCooldown") - 1;
    if cooldown > 0 {
        put(&mut e.nbt, "TransferCooldown", Tag::Int(cooldown));
        return;
    }
    put(&mut e.nbt, "TransferCooldown", Tag::Int(0));
    if prop(game, pos, "enabled").is_some_and(|v| v == "false") {
        return;
    }
    // The block's own space and the half block beyond its opening.
    let facing = prop(game, pos, "facing").unwrap_or_else(|| "up".into());
    let (dx, dz) = step(&facing);
    let dy = match facing.as_str() {
        "up" => 1,
        "down" => -1,
        _ => 0,
    };
    let p = pos.map(f64::from);
    let grow = |d: i32, lo: f64| if d < 0 { (lo - 0.5, lo + 1.0) } else if d > 0 { (lo, lo + 1.5) } else { (lo, lo + 1.0) };
    let (x0, x1) = grow(dx, p[0]);
    let (y0, y1) = grow(dy, p[1]);
    let (z0, z1) = grow(dz, p[2]);
    let mut took = false;
    let mut i = 0;
    while i < game.dropped.len() {
        let d = &game.dropped[i];
        let q = d.position;
        if d.pickup_delay > 0 || q.x < x0 || q.x > x1 || q.y < y0 || q.y > y1 || q.z < z0 || q.z > z1 {
            i += 1;
            continue;
        }
        let mut stack = d.stack.clone();
        let max = max_stack(game, &stack.item);
        for slot in e.items.iter_mut() {
            if stack.count == 0 {
                break;
            }
            match slot {
                None => {
                    *slot = Some(stack.clone());
                    stack.count = 0;
                }
                Some(s) if s.item == stack.item && s.extra == stack.extra && s.damage == stack.damage && s.count < max => {
                    let n = stack.count.min(max - s.count);
                    s.count += n;
                    stack.count -= n;
                }
                _ => {}
            }
        }
        if stack.count == 0 {
            game.remove_dropped(i);
            took = true;
            continue;
        }
        if stack.count != game.dropped[i].stack.count {
            game.dropped[i].stack = stack;
            took = true;
        }
        i += 1;
    }
    if took {
        put(&mut e.nbt, "TransferCooldown", Tag::Int(8));
    }
}

// ---------------------------------------------------------------------------------------------
// Drawing (client)
// ---------------------------------------------------------------------------------------------

/// `StoveBlockEntity.getStoveItemOffset`.
const STOVE_OFFSETS: [[f32; 2]; 6] = [[0.3, 0.2], [0.0, 0.2], [-0.3, 0.2], [0.3, -0.2], [0.0, -0.2], [-0.3, -0.2]];

pub(crate) fn submit(game: &Game<'_>, pos: [i32; 3], state: BlockStateId, entity: &ModBlockEntity, draw: &mut BlockEntityDraw) {
    use horizon_assets::item_model::DisplayContext;
    let Some(k) = kind_of_entity(&entity.id) else { return };
    let facing = game.native_block_state_property(state, "facing").unwrap_or("north").to_owned();
    let seed = pos[0].wrapping_mul(31).wrapping_add(pos[1]).wrapping_mul(31).wrapping_add(pos[2]);
    let y = [0.0, 1.0, 0.0];
    let x = [1.0, 0.0, 0.0];
    let o = [0.0; 3];
    match k {
        // StoveRenderer
        Kind::Stove => {
            let dir = opposite(&facing);
            for (i, s) in client_items(entity, 6).iter().enumerate() {
                let Some(s) = s else { continue };
                draw.push();
                draw.translate([0.5, 1.02, 0.5]);
                draw.rotate_about(y, -y_rot(dir), o);
                draw.rotate_about(x, 90.0, o);
                draw.translate([STOVE_OFFSETS[i][0], STOVE_OFFSETS[i][1], 0.0]);
                draw.scale([0.375; 3]);
                game.native_draw_item(draw, s, DisplayContext::Fixed, None, seed.wrapping_add(i as i32));
                draw.pop();
            }
            draw.keep();
        }
        // SkilletRenderer (without the flip)
        Kind::Skillet => {
            let Some(s) = client_items(entity, 1).into_iter().next().flatten() else {
                draw.keep();
                return;
            };
            let n = match s.count {
                49.. => 5,
                33.. => 4,
                17.. => 3,
                2.. => 2,
                _ => 1,
            };
            let mut r = horizon_core::random::LegacyRandom::new(i64::from(s.item.len() as i32 * 7919 + s.damage as i32));
            for i in 0..n {
                let xo = (r.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
                let zo = (r.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
                draw.push();
                draw.translate([0.5 + xo, 0.1 + 0.03 * (i as f32 + 1.0), 0.5 + zo]);
                draw.rotate_about(y, -y_rot(&facing), o);
                draw.rotate_about(x, 90.0, o);
                draw.scale([0.5; 3]);
                game.native_draw_item(draw, &s, DisplayContext::Fixed, None, seed);
                draw.pop();
            }
            draw.keep();
        }
        // CuttingBoardRenderer
        Kind::Board => {
            let Some(s) = client_items(entity, 1).into_iter().next().flatten() else {
                draw.keep();
                return;
            };
            let dir = opposite(&facing);
            let carved = nbt_int(&entity.nbt, "IsItemCarved") != 0;
            let block = game.pack.assets.registry.default_state(&s.item).is_some()
                && !game.items().item_in_tag("farmersdelight:flat_on_cutting_board", &s.item);
            draw.push();
            if carved {
                draw.translate([0.5, 0.23, 0.5]);
                draw.rotate_about(y, -y_rot(dir) + 180.0, o);
                let tags = game.items();
                let angle = if tags.item_in_tag("minecraft:pickaxes", &s.item) || tags.item_in_tag("minecraft:hoes", &s.item) {
                    225.0
                } else if &*s.item == "minecraft:trident" {
                    135.0
                } else {
                    180.0
                };
                draw.rotate_about([0.0, 0.0, 1.0], angle, o);
                draw.scale([0.6; 3]);
            } else if block {
                draw.translate([0.5, 0.27, 0.5]);
                draw.rotate_about(y, -y_rot(dir), o);
                draw.scale([0.8; 3]);
            } else {
                draw.translate([0.5, 0.08, 0.5]);
                draw.rotate_about(y, -y_rot(dir), o);
                draw.rotate_about(x, 90.0, o);
                draw.scale([0.6; 3]);
            }
            game.native_draw_item(draw, &s, DisplayContext::Fixed, None, seed);
            draw.pop();
            draw.keep();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// The cooking pot's screen (CookingPotMenu, CookingPotScreen)
// ---------------------------------------------------------------------------------------------

pub(crate) fn screen(block: &str) -> Option<horizon_client::mod_api::NativeScreen> {
    if kind_of_block(block) != Some(Kind::Pot) {
        return None;
    }
    let mut slots = Vec::new();
    for row in 0..2 {
        for col in 0..3 {
            slots.push(Some([30.0 + col as f32 * 18.0, 17.0 + row as f32 * 18.0]));
        }
    }
    slots.push(Some([124.0, 26.0]));
    slots.push(Some([92.0, 55.0]));
    slots.push(Some([124.0, 55.0]));
    Some(horizon_client::mod_api::NativeScreen {
        width: 176.0,
        height: 166.0,
        slots,
        inventory_y: 84.0,
        inventory_x: 8.0,
        own_labels: false,
        own_background: true,
    })
}

pub(crate) fn screen_draw(game: &Game<'_>, block: &str, data: &[i32], batch: &mut HudBatch, origin: [f32; 2]) {
    if kind_of_block(block) != Some(Kind::Pot) {
        return;
    }
    let [ox, oy] = origin;
    let tex = "farmersdelight:gui/cooking_pot";
    game.native_sprite(batch, tex, [0.0, 0.0, 176.0, 166.0], [ox, oy, 176.0, 166.0]);
    let get = |i: usize| data.get(i).copied().unwrap_or(0);
    if get(2) != 0 {
        game.native_sprite(batch, tex, [176.0, 0.0, 17.0, 15.0], [ox + 47.0, oy + 55.0, 17.0, 15.0]);
    }
    let (cook, total) = (get(0), get(1));
    let l = if cook != 0 && total != 0 { cook * 24 / total } else { 0 };
    if l > 0 {
        let w = (l + 1) as f32;
        game.native_sprite(batch, tex, [176.0, 15.0, w, 17.0], [ox + 89.0, oy + 25.0, w, 17.0]);
    }
}

// ---------------------------------------------------------------------------------------------
// Cabinets open and close (CabinetBlockEntity.ContainerOpenersCounter)
// ---------------------------------------------------------------------------------------------

pub(crate) fn viewers(game: &mut Game<'_>, pos: [i32; 3], viewers: u32, previous: u32) {
    if kind_of_block(&name_at(game, pos)) != Some(Kind::Cabinet) {
        return;
    }
    let open = viewers > 0;
    if open == (previous > 0) {
        return;
    }
    set_props(game, pos, &[("open", if open { "true" } else { "false" })]);
    let facing = prop(game, pos, "facing").unwrap_or_else(|| "north".into());
    let (dx, dz) = step(&facing);
    let at = [
        f64::from(pos[0]) + 0.5 + f64::from(dx) * 0.5,
        f64::from(pos[1]) + 0.5,
        f64::from(pos[2]) + 0.5 + f64::from(dz) * 0.5,
    ];
    let pitch = game.world.random.next_float() * 0.1 + 0.9;
    let sound = if open { "farmersdelight:block.cabinet.open" } else { "farmersdelight:block.cabinet.close" };
    play(game, sound, at, 0.5, pitch);
}

// ---------------------------------------------------------------------------------------------
// Right clicks on blocks
// ---------------------------------------------------------------------------------------------

/// Runs `f` on the block entity at `p`.
fn with_entity<R>(game: &mut Game<'_>, p: [i32; 3], f: impl FnOnce(&mut ModBlockEntity) -> R) -> Option<R> {
    game.native_with_mod_block_entity(p, f)
}
fn entity_items(game: &mut Game<'_>, p: [i32; 3], n: usize) -> Option<Vec<Option<ItemStack>>> {
    with_entity(game, p, |m| {
        let mut v = m.items.clone();
        sized(&mut v, n);
        v
    })
}

/// `use` of Farmer's Delight's blocks: true when the click is taken (nothing else acts).
pub(crate) fn right_click(game: &mut Game<'_>, at: [i32; 3], _side: [i32; 3]) -> bool {
    let name = name_at(game, at);
    let Some(n) = short(&name) else { return false };
    let main = game.hand == horizon_client::first_person::Hand::Main;
    let sneak = sneaking(game);
    let held = game.held().cloned();
    // Blocks whose template would act on its own (a furnace's menu, a cake's bite) take every
    // click their Java block would not use.
    let template_acts = n == "stove" || is_pie(n) || feast(n).is_some();
    if !main {
        return template_acts;
    }
    // CuttingBoardBlock.ToolCarvingEvent: sneaking with a tool carves it onto an empty board.
    if n == "cutting_board" && sneak {
        return held.is_some_and(|h| board_carve(game, at, &h));
    }
    // isSecondaryUseActive with something in a hand: the block isn't used.
    let hands_full = game.stack_in(game.selected).is_some() || game.offhand.is_some();
    if sneak && hands_full {
        return false;
    }
    match n {
        "stove" => {
            stove_use(game, at, held.as_ref());
            true
        }
        "cooking_pot" => pot_use(game, at, held.as_ref(), sneak),
        "skillet" => skillet_use(game, at, held.as_ref()),
        "cutting_board" => board_use(game, at, held.as_ref()),
        "tomatoes" => tomato_use(game, at, held.as_ref()),
        "brown_mushroom_colony" | "red_mushroom_colony" => colony_use(game, at, n, held.as_ref()),
        _ if is_pie(n) => {
            pie_use(game, at, n, held.as_ref());
            true
        }
        _ if feast(n).is_some() => {
            feast_use(game, at, n, held.as_ref());
            true
        }
        _ => false,
    }
}

/// `StoveBlock.use`.
fn stove_use(game: &mut Game<'_>, at: [i32; 3], held: Option<&ItemStack>) {
    let c = centre(at);
    let lit = prop(game, at, "lit").is_some_and(|v| v == "true");
    if let Some(h) = held {
        let item = &*h.item;
        if lit {
            if item_tag(game, "minecraft:shovels", item) {
                set_props(game, at, &[("lit", "false")]);
                play(game, "minecraft:block.fire.extinguish", c, 0.5, 2.6);
                game.damage_held(1);
                return;
            }
            if item == "minecraft:water_bucket" {
                play(game, "minecraft:entity.generic.extinguish_fire", c, 1.0, 1.0);
                set_props(game, at, &[("lit", "false")]);
                play(game, "minecraft:block.fire.extinguish", c, 0.5, 2.6);
                if !game.creative {
                    *game.held_mut() = Some(stack_of("minecraft:bucket", 1));
                }
                return;
            }
        } else {
            if item == "minecraft:flint_and_steel" {
                let pitch = game.world.random.next_float() * 0.4 + 0.8;
                play(game, "minecraft:item.flintandsteel.use", c, 1.0, pitch);
                set_props(game, at, &[("lit", "true")]);
                game.damage_held(1);
                return;
            }
            if item == "minecraft:fire_charge" {
                let r = &mut game.world.random;
                let pitch = (r.next_float() - r.next_float()) * 0.2 + 1.0;
                play(game, "minecraft:item.firecharge.use", c, 1.0, pitch);
                set_props(game, at, &[("lit", "true")]);
                use_one(game);
                return;
            }
        }
    }
    let Some(h) = held else { return };
    let Some(items) = entity_items(game, at, 6) else { return };
    let Some(slot) = items.iter().position(Option::is_none) else { return };
    if stove_blocked(game, at) {
        return;
    }
    let Some((_, time)) = campfire(game, h) else { return };
    let one = ItemStack { count: 1, ..h.clone() };
    with_entity(game, at, |m| {
        sized(&mut m.items, 6);
        m.items[slot] = Some(one);
        let mut totals = nbt_ints(&m.nbt, "CookingTotalTimes", 6);
        let mut times = nbt_ints(&m.nbt, "CookingTimes", 6);
        totals[slot] = time as i32;
        times[slot] = 0;
        put(&mut m.nbt, "CookingTotalTimes", Tag::IntArray(totals));
        put(&mut m.nbt, "CookingTimes", Tag::IntArray(times));
    });
    use_one(game);
    game.native_block_entity_sync(at);
}

/// `CookingPotBlock.use`: the handle, or a serving into the held container. False lets the
/// pot's menu open.
fn pot_use(game: &mut Game<'_>, at: [i32; 3], held: Option<&ItemStack>, sneak: bool) -> bool {
    let c = centre(at);
    let Some(h) = held else {
        if sneak {
            let support = prop(game, at, "support").unwrap_or_default();
            let next = if support == "handle" {
                if tray_state(game, at) { "tray" } else { "none" }
            } else {
                "handle"
            };
            set_props(game, at, &[("support", next)]);
            play(game, "minecraft:block.lantern.place", c, 0.7, 1.0);
            return true;
        }
        return false;
    };
    // useHeldItemOnMeal
    let Some((meal, container)) = with_entity(game, at, |m| {
        sized(&mut m.items, 9);
        (m.items[MEAL].clone(), nbt_str(&m.nbt, "Container"))
    }) else {
        return false;
    };
    let Some(meal) = meal else { return false };
    let valid = match &container {
        Some(c) => *h.item == **c,
        None => h.item == meal.item,
    };
    if !valid {
        return false;
    }
    with_entity(game, at, |m| {
        if let Some(s) = &mut m.items[MEAL] {
            s.count -= 1;
            if s.count == 0 {
                m.items[MEAL] = None;
            }
        }
    });
    use_one(game);
    give_or_drop(game, ItemStack { count: 1, ..meal });
    play(game, "minecraft:item.armor.equip_generic", c, 1.0, 1.0);
    true
}

/// `SkilletBlock.use`.
fn skillet_use(game: &mut Game<'_>, at: [i32; 3], held: Option<&ItemStack>) -> bool {
    let c = centre(at);
    let Some(items) = entity_items(game, at, 1) else { return false };
    let stored = items[0].clone();
    let Some(h) = held else {
        // An empty hand takes the whole stack.
        let Some(s) = stored else { return true };
        with_entity(game, at, |m| m.items[0] = None);
        if !game.creative {
            *game.held_mut() = Some(s);
        }
        game.native_block_entity_sync(at);
        return true;
    };
    let is_heated = heated(game, at, true);
    if item_tag(game, "minecraft:shovels", &h.item) && is_heated && stored.is_some() {
        let now = game.world.game_time;
        let flipped = with_entity(game, at, |m| {
            if nbt_long(&m.nbt, "FlipTime") + 12 < now {
                put(&mut m.nbt, "FlipTime", Tag::Long(now));
                true
            } else {
                false
            }
        });
        if flipped == Some(true) {
            game.native_block_entity_sync(at);
        }
        return true;
    }
    // addItemToCook
    let Some((_, time)) = campfire(game, h) else {
        tell(game, horizon_client::text::Text::translate("farmersdelight.block.skillet.invalid_item", &[]));
        return false;
    };
    let max = max_stack(game, &h.item);
    let fits = match &stored {
        None => h.count.min(max),
        Some(s) if s.item == h.item && s.extra == h.extra && s.damage == h.damage => h.count.min(max.saturating_sub(s.count)),
        Some(_) => 0,
    };
    if fits == 0 {
        return false;
    }
    let was_empty = stored.is_none();
    let total = skillet_time(time);
    let added = ItemStack { count: stored.as_ref().map_or(0, |s| s.count) + fits, ..h.clone() };
    with_entity(game, at, |m| {
        m.items[0] = Some(added);
        put(&mut m.nbt, "CookTimeTotal", Tag::Int(total));
        put(&mut m.nbt, "CookTime", Tag::Int(0));
    });
    if was_empty && is_heated {
        play(game, "farmersdelight:block.skillet.add_food", c, 0.8, 1.0);
    }
    if !game.creative {
        let slot = game.held_mut();
        if let Some(s) = slot {
            s.count -= fits;
            if s.count == 0 {
                *slot = None;
            }
        }
    }
    play(game, "minecraft:block.lantern.place", c, 0.7, 1.0);
    game.native_block_entity_sync(at);
    true
}

/// Tools that can be carved onto a board: `TieredItem`, `TridentItem`, `ShearsItem`.
fn is_tool(game: &Game<'_>, item: &str) -> bool {
    let tags = game.items();
    ["minecraft:swords", "minecraft:axes", "minecraft:pickaxes", "minecraft:shovels", "minecraft:hoes"]
        .iter()
        .any(|t| tags.item_in_tag(t, item))
        || is_knife(game, item)
        || item == "minecraft:trident"
        || is_shears(game, item)
}

/// `CuttingBoardBlockEntity.carveToolOnBoard`.
fn board_carve(game: &mut Game<'_>, at: [i32; 3], held: &ItemStack) -> bool {
    if !is_tool(game, &held.item) {
        return false;
    }
    let Some(items) = entity_items(game, at, 1) else { return false };
    if items[0].is_some() {
        return false;
    }
    let one = ItemStack { count: 1, ..held.clone() };
    with_entity(game, at, |m| {
        sized(&mut m.items, 1);
        m.items[0] = Some(one);
        put(&mut m.nbt, "IsItemCarved", Tag::Byte(1));
    });
    use_one(game);
    play(game, "minecraft:block.wood.place", at.map(f64::from), 1.0, 0.8);
    game.native_block_entity_sync(at);
    true
}

/// `CuttingBoardBlock.use`.
fn board_use(game: &mut Game<'_>, at: [i32; 3], held: Option<&ItemStack>) -> bool {
    let Some(items) = entity_items(game, at, 1) else { return false };
    let corner = at.map(f64::from);
    match (&items[0], held) {
        (None, None) => false,
        (None, Some(h)) => {
            // An off hand item that isn't equipment keeps the main hand's item off the board,
            // unless that is a block.
            if let Some(off) = game.offhand.clone()
                && !item_tag(game, "farmersdelight:offhand_equipment", &off.item)
                && game.pack.assets.registry.default_state(&h.item).is_none()
            {
                return false;
            }
            let one = ItemStack { count: 1, ..h.clone() };
            with_entity(game, at, |m| {
                sized(&mut m.items, 1);
                m.items[0] = Some(one);
                put(&mut m.nbt, "IsItemCarved", Tag::Byte(0));
            });
            use_one(game);
            play(game, "minecraft:block.wood.place", corner, 1.0, 0.8);
            game.native_block_entity_sync(at);
            true
        }
        (Some(stored), Some(tool)) => {
            board_cut(game, at, stored.clone(), tool.clone());
            true
        }
        (Some(stored), None) => {
            let stored = stored.clone();
            with_entity(game, at, |m| m.items[0] = None);
            if !game.creative {
                give_or_drop(game, stored);
            }
            play(game, "minecraft:block.wood.hit", corner, 0.25, 0.5);
            game.native_block_entity_sync(at);
            true
        }
    }
}

/// `CuttingBoardRecipe.getTool().test`: an item tag or a tool action.
fn tool_matches(game: &Game<'_>, tool: &serde_json::Value, item: &str) -> bool {
    if let Some(t) = tool.as_str() {
        return match t.strip_prefix('#') {
            Some("c:shears") => is_shears(game, item),
            Some("c:tools/knives") => is_knife(game, item),
            Some(tag) => item_tag(game, tag, item),
            None => t == item,
        };
    }
    if let Some(t) = tool["tag"].as_str() {
        return item_tag(game, t, item);
    }
    if let Some(i) = tool["item"].as_str() {
        return i == item;
    }
    let tags = game.items();
    match tool["action"].as_str().unwrap_or("") {
        "axe_dig" | "axe_strip" => tags.item_in_tag("minecraft:axes", item),
        "pickaxe_dig" => tags.item_in_tag("minecraft:pickaxes", item),
        "shovel_dig" | "shovel_flatten" => tags.item_in_tag("minecraft:shovels", item),
        "hoe_dig" | "hoe_till" => tags.item_in_tag("minecraft:hoes", item),
        "shears_dig" | "shears_harvest" => is_shears(game, item),
        "sword_dig" => tags.item_in_tag("minecraft:swords", item),
        _ => false,
    }
}

/// `CuttingBoardBlockEntity.processStoredItemUsingTool`.
fn board_cut(game: &mut Game<'_>, at: [i32; 3], stored: ItemStack, tool: ItemStack) {
    let carved = with_entity(game, at, |m| nbt_int(&m.nbt, "IsItemCarved") != 0).unwrap_or(false);
    if carved {
        return;
    }
    let recipes = game.native_recipes_of_type("farmersdelight:cutting").to_vec();
    let mut any = false;
    let mut found = None;
    for r in &recipes {
        let j = &r.json;
        let Some(first) = j["ingredients"].get(0).or(j["ingredient"].get(0)) else { continue };
        let Some(ing) = game.native_recipe_ingredient(first) else { continue };
        if !ing.test(&stored) {
            continue;
        }
        any = true;
        if tool_matches(game, &j["tool"], &tool.item) {
            found = Some(Arc::clone(&r.json));
            break;
        }
    }
    let Some(j) = found else {
        let key = if any { "farmersdelight.block.cutting_board.invalid_tool" } else { "farmersdelight.block.cutting_board.invalid_item" };
        tell(game, horizon_client::text::Text::translate(key, &[]));
        return;
    };
    let facing = prop(game, at, "facing").unwrap_or_else(|| "north".into());
    let (dx, dz) = step(counter_clockwise(&facing));
    for res in j["result"].as_array().into_iter().flatten() {
        let Some(id) = res["id"].as_str().or(res["item"].as_str()) else { continue };
        let count = res["count"].as_u64().unwrap_or(1) as u32;
        let chance = res["chance"].as_f64().unwrap_or(1.0);
        let mut out = count;
        for _ in 0..count {
            if f64::from(game.world.random.next_float()) > chance + FORTUNE_BONUS * 0.0 {
                out -= 1;
            }
        }
        if out == 0 {
            continue;
        }
        let item = horizon_client::atlas_sources::qualified(id);
        let spot = [
            f64::from(at[0]) + 0.5 + f64::from(dx) * 0.2,
            f64::from(at[1]) + 0.2,
            f64::from(at[2]) + 0.5 + f64::from(dz) * 0.2,
        ];
        spawn(game, stack_of(&item, out), spot, [f64::from(dx) * 0.2, 0.0, f64::from(dz) * 0.2]);
    }
    game.damage_held(1);
    // playProcessingSound
    let c = centre(at);
    if let Some(s) = j["sound"].as_str().or(j["sound"]["sound_id"].as_str()) {
        play(game, s, c, 1.0, 1.0);
    } else if is_shears(game, &tool.item) {
        play(game, "minecraft:entity.sheep.shear", c, 1.0, 1.0);
    } else if is_knife(game, &tool.item) {
        play(game, "farmersdelight:block.cutting_board.knife", c, 0.8, 1.0);
    } else if let Some(state) = game.pack.assets.registry.default_state(&stored.item) {
        let sound = game.res.facts().get(state).sounds.break_sound.clone();
        play(game, &sound, c, 1.0, 0.8);
    } else {
        play(game, "minecraft:block.wood.break", c, 1.0, 0.8);
    }
    with_entity(game, at, |m| m.items[0] = None);
    game.native_block_entity_sync(at);
}

/// `TomatoVineBlock.use`: picking ripe tomatoes.
fn tomato_use(game: &mut Game<'_>, at: [i32; 3], _held: Option<&ItemStack>) -> bool {
    let age: i32 = prop(game, at, "age").and_then(|v| v.parse().ok()).unwrap_or(0);
    if age < 3 {
        return false;
    }
    let n = 1 + game.world.random.next_int_bound(2).unwrap_or(0) as u32;
    pop(game, at, stack_of("farmersdelight:tomato", n));
    if game.world.random.next_float() < 0.05 {
        pop(game, at, stack_of("farmersdelight:rotten_tomato", 1));
    }
    let pitch = 0.8 + game.world.random.next_float() * 0.4;
    play(game, "farmersdelight:block.tomato_bush.pick_tomatoes", centre(at), 1.0, pitch);
    if let Some(s) = with_props(game, at, &[("age", "0")]) {
        let _ = game.level_set_with_flags(at, s, 2);
    }
    true
}

/// `MushroomColonyBlock.use`: shears take a mushroom.
fn colony_use(game: &mut Game<'_>, at: [i32; 3], n: &str, held: Option<&ItemStack>) -> bool {
    let age: i32 = prop(game, at, "age").and_then(|v| v.parse().ok()).unwrap_or(0);
    if age == 0 || !held.is_some_and(|h| &*h.item == "minecraft:shears") {
        return false;
    }
    let mushroom = if n.starts_with("brown") { "minecraft:brown_mushroom" } else { "minecraft:red_mushroom" };
    pop(game, at, stack_of(mushroom, 1));
    play(game, "minecraft:entity.mooshroom.shear", centre(at), 1.0, 1.0);
    if let Some(s) = with_props(game, at, &[("age", &(age - 1).to_string())]) {
        let _ = game.level_set_with_flags(at, s, 2);
    }
    game.damage_held(1);
    true
}

fn is_pie(n: &str) -> bool {
    matches!(n, "apple_pie" | "sweet_berry_cheesecake" | "chocolate_pie")
}

/// `PieBlock.use`: a knife cuts a slice, anything else eats one.
fn pie_use(game: &mut Game<'_>, at: [i32; 3], n: &str, held: Option<&ItemStack>) {
    let slice = format!("{NS}{n}_slice");
    let bites: i32 = prop(game, at, "bites").and_then(|v| v.parse().ok()).unwrap_or(0);
    let c = centre(at);
    let knife = held.is_some_and(|h| is_knife(game, &h.item));
    if !knife {
        // consumeBite
        let Some(food) = game.items().get(&slice).and_then(|i| i.food) else { return };
        let hungry = game.player.as_ref().is_some_and(|p| p.health.food.food < 20);
        if !game.creative && !hungry {
            return;
        }
        if let Some(p) = game.player.as_mut() {
            let f = &mut p.health.food;
            f.food = (f.food + food.0 as i32).clamp(0, 20);
            f.saturation = (f.saturation + food.1).clamp(0.0, f.food as f32);
        }
    }
    if bites < 3 {
        set_props(game, at, &[("bites", &(bites + 1).to_string())]);
    } else {
        remove_block(game, at);
    }
    if knife {
        let yaw = game.player.as_ref().map_or(0.0, |p| p.yaw);
        let (dx, dz) = step(opposite(horizontal(yaw)));
        let spot = [c[0], f64::from(at[1]) + 0.3, c[2]];
        spawn(game, stack_of(&slice, 1), spot, [f64::from(dx) * 0.15, 0.05, f64::from(dz) * 0.15]);
        play(game, "minecraft:block.wool.break", c, 0.8, 0.8);
    } else {
        play(game, "minecraft:entity.generic.eat", c, 0.8, 0.8);
    }
}

/// A feast's serving item and whether it leaves its platter behind.
fn feast(n: &str) -> Option<(&'static str, bool)> {
    Some(match n {
        "roast_chicken_block" => ("farmersdelight:roast_chicken", true),
        "stuffed_pumpkin_block" => ("farmersdelight:stuffed_pumpkin", false),
        "honey_glazed_ham_block" => ("farmersdelight:honey_glazed_ham", true),
        "shepherds_pie_block" => ("farmersdelight:shepherds_pie", true),
        "rice_roll_medley_block" => ("farmersdelight:salmon_roll", true),
        _ => return None,
    })
}
/// `RiceRollMedleyBlock.riceRollServings`.
const MEDLEY: [&str; 8] = [
    "farmersdelight:cod_roll",
    "farmersdelight:cod_roll",
    "farmersdelight:salmon_roll",
    "farmersdelight:salmon_roll",
    "farmersdelight:salmon_roll",
    "farmersdelight:kelp_roll_slice",
    "farmersdelight:kelp_roll_slice",
    "farmersdelight:kelp_roll_slice",
];

/// `FeastBlock.takeServing`.
fn feast_use(game: &mut Game<'_>, at: [i32; 3], n: &str, held: Option<&ItemStack>) {
    let Some((serving, leftovers)) = feast(n) else { return };
    let c = centre(at);
    let servings: i32 = prop(game, at, "servings").and_then(|v| v.parse().ok()).unwrap_or(0);
    if servings == 0 {
        play(game, "minecraft:block.wood.break", c, 0.8, 0.8);
        let _ = game.destroy_block_dropping(at);
        return;
    }
    let serving = if n == "rice_roll_medley_block" { MEDLEY[(servings as usize - 1).min(7)] } else { serving };
    let container = remainder(game, serving);
    let ok = container.as_ref().is_none_or(|c| held.is_some_and(|h| *h.item == **c));
    if !ok {
        let c = container.unwrap_or_default();
        let item_name = horizon_client::text::Text::translate(&format!("item.{}", c.replace(':', ".")), &[]);
        tell(game, horizon_client::text::Text::translate("farmersdelight.block.feast.use_container", &[item_name]));
        return;
    }
    set_props(game, at, &[("servings", &(servings - 1).to_string())]);
    if container.is_some() {
        use_one(game);
    }
    give_or_drop(game, stack_of(serving, 1));
    if servings - 1 == 0 && !leftovers {
        remove_block(game, at);
    }
    play(game, "minecraft:item.armor.equip_generic", c, 1.0, 1.0);
}

// ---------------------------------------------------------------------------------------------
// Items used on blocks
// ---------------------------------------------------------------------------------------------

/// The crop a seed item plants (an item whose block has another id).
fn seed_crop(item: &str) -> Option<&'static str> {
    Some(match item {
        "farmersdelight:cabbage_seeds" => "farmersdelight:cabbages",
        "farmersdelight:tomato_seeds" => "farmersdelight:budding_tomatoes",
        "farmersdelight:onion" => "farmersdelight:onions",
        _ => return None,
    })
}

/// `useOn` of Farmer's Delight's items, and hoes on rich soil: true when taken.
pub(crate) fn use_on(game: &mut Game<'_>, item: &ItemStack, at: [i32; 3], side: [i32; 3]) -> bool {
    let name = name_at(game, at);
    let above = [at[0], at[1] + 1, at[2]];
    // RichSoilBlock.getToolModifiedState: a hoe tills it, with air above.
    if &*name == "farmersdelight:rich_soil" && item_tag(game, "minecraft:hoes", &item.item) && side != [0, -1, 0] {
        if !game.res.facts().is_air(state_at(game, above)) {
            return false;
        }
        set_default(game, at, "farmersdelight:rich_soil_farmland", &[]);
        play(game, "minecraft:item.hoe.till", centre(at), 1.0, 1.0);
        game.damage_held(1);
        return true;
    }
    // ItemNameBlockItem seeds: on the top of farmland.
    if let Some(crop) = seed_crop(&item.item) {
        let ground_ok = matches!(&*name, "minecraft:farmland" | "farmersdelight:rich_soil_farmland");
        if side != [0, 1, 0] || !ground_ok || !game.res.facts().is_air(state_at(game, above)) {
            return false;
        }
        if !set_default(game, above, crop, &[]) {
            return false;
        }
        play(game, "minecraft:item.crop.plant", centre(above), 1.0, 1.0);
        use_one(game);
        return true;
    }
    // RopeItem.updatePlacementContext: a rope on a rope goes to the bottom of it.
    if &*item.item == "farmersdelight:rope" && &*name == "farmersdelight:rope" && !sneaking(game) {
        let mut p = at;
        for _ in 0..256 {
            p[1] -= 1;
            let below = name_at(game, p);
            if &*below == "farmersdelight:rope" {
                continue;
            }
            let state = state_at(game, p);
            let facts = game.res.facts().get(state);
            if facts.fluid == horizon_client::blocks::Fluid::Lava {
                return true;
            }
            if facts.replaceable && set_default(game, p, "farmersdelight:rope", &[]) {
                play(game, "minecraft:block.wool.place", centre(p), 1.0, 0.8);
                use_one(game);
            }
            return true;
        }
        return true;
    }
    false
}

/// `BlockItem.place` heard: the tatami's pair and the full mat's other half
/// (`TatamiBlock.setPlacedBy`, `TatamiMatBlock.setPlacedBy`).
pub(crate) fn placed(game: &mut Game<'_>, pos: [i32; 3], state: BlockStateId) {
    let name = game.res.facts().get(state).name.clone();
    match short(&name) {
        Some("full_tatami_mat") => {
            let facing = prop(game, pos, "facing").unwrap_or_else(|| "north".into());
            let (dx, dz) = step(&facing);
            let head = [pos[0] + dx, pos[1], pos[2] + dz];
            if game.res.facts().get(state_at(game, head)).replaceable {
                set_default(game, head, "farmersdelight:full_tatami_mat", &[("facing", &facing), ("part", "head")]);
            }
        }
        Some("tatami") if !sneaking(game) => {
            let facing = prop(game, pos, "facing").unwrap_or_else(|| "down".into());
            let d: [i32; 3] = match facing.as_str() {
                "up" => [0, 1, 0],
                "down" => [0, -1, 0],
                f => {
                    let (x, z) = step(f);
                    [x, 0, z]
                }
            };
            let other = [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]];
            if &*name_at(game, other) == "farmersdelight:tatami" && prop(game, other, "paired").as_deref() == Some("false") {
                set_props(game, other, &[("facing", opposite(&facing)), ("paired", "true")]);
                set_props(game, pos, &[("paired", "true")]);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// Foods and effects
// ---------------------------------------------------------------------------------------------

/// The drinks' and the popsicle's own effects (`affectConsumer`).
pub(crate) fn consume_effect(game: &mut Game<'_>, kind: &str) {
    match kind {
        // MilkBottleItem, HotCocoaItem: one random effect goes (a harmful one, for cocoa).
        "farmersdelight:cure_one" | "farmersdelight:cure_one_harmful" => {
            let harmful_only = kind.ends_with("harmful");
            let ids: Vec<&'static str> = game
                .potions
                .effects
                .list
                .iter()
                .map(|e| e.id)
                .filter(|id| {
                    !harmful_only
                        || horizon_client::brewing::effect(id)
                            .is_some_and(|d| d.category == horizon_client::brewing::Category::Harmful)
                })
                .collect();
            if ids.is_empty() {
                return;
            }
            let pick = ids[game.world.random.next_int_bound(ids.len() as i32).unwrap_or(0) as usize];
            game.potions.effects.list.retain(|e| e.id != pick);
            game.potions.effects.changes = game.potions.effects.changes.wrapping_add(1);
            game.apply_effect_status();
        }
        // MelonJuiceItem
        "farmersdelight:heal" => game.apply_effect_action(EffectAction::Heal(2.0)),
        // PopsicleItem
        "farmersdelight:extinguish" => {
            if let Some(p) = game.player.as_mut() {
                p.fire_ticks = p.fire_ticks.min(0);
            }
        }
        _ => {}
    }
}

/// Comfort and Nourishment (`ComfortEffect`, `NourishmentEffect`), and the lit stove's burn
/// (`StoveBlock.stepOn`), for the player whose effects tick now.
pub(crate) fn player_tick(game: &mut Game<'_>) {
    if game.player.as_ref().is_none_or(|p| p.health.dead_or_dying()) {
        return;
    }
    // ComfortEffect: 1 health every 80 ticks (80 >> amplifier), without regeneration or saturation.
    let comfort = game.potions.effects.get("farmersdelight:comfort").map(|e| (e.amplifier, e.duration));
    let regen = game.potions.effects.get("regeneration").is_some();
    if let Some((amp, duration)) = comfort
        && !regen
        && let Some(p) = game.player.as_mut()
    {
        let k = 80 >> amp.clamp(0, 31);
        let due = k <= 0 || duration % k == 0;
        let max = p.attribute("minecraft:max_health").unwrap_or(20.0) as f32;
        if due && p.health.food.saturation <= 0.0 && p.health.health() < max {
            game.apply_effect_action(EffectAction::Heal(1.0));
        }
    }
    // NourishmentEffect: no exhaustion, unless natural regeneration is healing on hunger.
    if game.potions.effects.get("farmersdelight:nourishment").is_some()
        && let Some(p) = game.player.as_mut()
    {
        let max = p.attribute("minecraft:max_health").unwrap_or(20.0) as f32;
        let healing = p.health.health() < max && p.health.food.food >= 18;
        if !healing {
            let f = &mut p.health.food;
            f.exhaustion = (f.exhaustion - f.exhaustion.min(4.0)).max(0.0);
        }
    }
    // StoveBlock.stepOn: standing on a lit stove burns.
    let Some(p) = game.player.as_ref() else { return };
    if game.creative || !p.ground.on_ground {
        return;
    }
    let under = [p.position.x.floor() as i32, (p.position.y - 0.2).floor() as i32, p.position.z.floor() as i32];
    if &*name_at(game, under) == "farmersdelight:stove" && prop(game, under, "lit").as_deref() == Some("true") {
        let frost = game.armor[3].as_ref().is_some_and(|s| s.extra.as_deref().is_some_and(|x| format!("{x:?}").contains("frost_walker")));
        if !frost {
            game.apply_effect_action(EffectAction::Hurt("minecraft:hot_floor", 1.0));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The level: random ticks, bone meal, survival, placement
// ---------------------------------------------------------------------------------------------

fn lname<'b>(b: &'b Behaviours, state: BlockStateId) -> &'b str {
    b.table.name(state)
}
fn lprop_int(b: &Behaviours, state: BlockStateId, k: &str) -> i32 {
    b.table.key_of(k).map_or(0, |key| b.table.int(state, key))
}
fn lprop_bool(b: &Behaviours, state: BlockStateId, k: &str) -> bool {
    b.table.key_of(k).is_some_and(|key| b.table.bool(state, key))
}
fn lwith_int(b: &Behaviours, state: BlockStateId, k: &str, v: i32) -> Option<BlockStateId> {
    b.table.with_int(state, b.table.key_of(k)?, v)
}
fn lwith_bool(b: &Behaviours, state: BlockStateId, k: &str, v: bool) -> Option<BlockStateId> {
    b.table.with_bool(state, b.table.key_of(k)?, v)
}
fn lwith_text(b: &Behaviours, state: BlockStateId, k: &str, v: &str) -> Option<BlockStateId> {
    b.table.with_text(state, b.table.key_of(k)?, v)
}
fn ldefault(b: &Behaviours, name: &str) -> Option<BlockStateId> {
    b.table.block_by_name(name).map(|id| b.table.default_state(id))
}
fn off(p: BlockPos, dx: i32, dy: i32, dz: i32) -> BlockPos {
    BlockPos::new(p.x + dx, p.y + dy, p.z + dz)
}
fn lnext_int(level: &mut GameLevel<'_>, bound: i32) -> i32 {
    level.random().next_int_bound(bound).unwrap_or(0)
}
/// `Mth.nextInt(random, a, b)`.
fn lbetween(level: &mut GameLevel<'_>, a: i32, b: i32) -> i32 {
    a + lnext_int(level, b - a + 1)
}
/// `Level.getRawBrightness(pos, 0)`.
fn brightness(level: &GameLevel<'_>, p: BlockPos) -> i32 {
    (level.sky_light(p) - level.sky_darken()).max(level.block_light(p))
}
fn sees_sky(level: &GameLevel<'_>, p: BlockPos) -> bool {
    level.sky_light(p) >= 15
}
fn in_block_tag(level: &GameLevel<'_>, tag: &str, state: BlockStateId) -> bool {
    level.items.block_in_tag(tag, lname(level.behaviours, state))
}
fn set(level: &mut GameLevel<'_>, ticks: &mut Ticks, p: BlockPos, state: BlockStateId, flags: i32) {
    let b = level.behaviours;
    b.set_block(level, ticks, p, state, flags);
}

/// `CropBlock.getGrowthSpeed` (and `BuddingBushBlock.getGrowthSpeed`, the same here).
fn growth_speed(level: &GameLevel<'_>, pos: BlockPos, block: &str) -> f32 {
    let b = level.behaviours;
    let mut speed = 1.0f32;
    for dx in -1..=1 {
        for dz in -1..=1 {
            let s = level.block_state(off(pos, dx, -1, dz));
            let n = lname(b, s);
            let mut bonus = 0.0;
            if n == "minecraft:farmland" || n == "farmersdelight:rich_soil_farmland" {
                bonus = 1.0;
                if lprop_int(b, s, "moisture") > 0 {
                    bonus = 3.0;
                }
            }
            if dx != 0 || dz != 0 {
                bonus /= 4.0;
            }
            speed += bonus;
        }
    }
    let is = |dx, dz| lname(b, level.block_state(off(pos, dx, 0, dz))) == block;
    let ew = is(-1, 0) || is(1, 0);
    let ns = is(0, -1) || is(0, 1);
    if (ew && ns) || is(-1, -1) || is(1, -1) || is(1, 1) || is(-1, 1) {
        speed /= 2.0;
    }
    speed
}

/// The rich soil boost: the plant above takes bone meal (`performBonemeal`) at 80%.
fn rich_boost(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, skip_tall_flowers: bool) {
    let above = off(pos, 0, 1, 0);
    let st = level.block_state(above);
    if in_block_tag(level, "farmersdelight:unaffected_by_rich_soil", st)
        || (skip_tall_flowers && in_block_tag(level, "minecraft:tall_flowers", st))
        || level.behaviours.is_air(st)
    {
        return;
    }
    if level.random().next_float() <= RICH_SOIL_BOOST {
        let b = level.behaviours;
        b.grow_crop(level, ticks, above);
    }
}

/// `randomTick` of Farmer's Delight's blocks: true when handled.
pub(crate) fn random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) -> bool {
    let b = level.behaviours;
    let name = lname(b, state).to_owned();
    let Some(n) = short(&name) else { return false };
    match n {
        // RichSoilBlock.randomTick
        "rich_soil" => {
            let above = off(pos, 0, 1, 0);
            let up = lname(b, level.block_state(above)).to_owned();
            if in_block_tag(level, "farmersdelight:unaffected_by_rich_soil", level.block_state(above)) {
                return true;
            }
            let colony = match up.as_str() {
                "minecraft:brown_mushroom" => "farmersdelight:brown_mushroom_colony",
                "minecraft:red_mushroom" => "farmersdelight:red_mushroom_colony",
                _ => {
                    rich_boost(level, ticks, pos, false);
                    return true;
                }
            };
            if let Some(s) = ldefault(b, colony) {
                set(level, ticks, above, s, 3);
            }
            true
        }
        // RichSoilFarmlandBlock.randomTick
        "rich_soil_farmland" => {
            let moisture = lprop_int(b, state, "moisture");
            let mut water = false;
            'scan: for dx in -4..=4 {
                for dy in 0..=1 {
                    for dz in -4..=4 {
                        let f = b.fluid(level.block_state(off(pos, dx, dy, dz)));
                        if f.kind().same_family(horizon_world::fluid::FluidKind::Water) {
                            water = true;
                            break 'scan;
                        }
                    }
                }
            }
            if !water && !level.is_raining_at(off(pos, 0, 1, 0)) {
                if moisture > 0
                    && let Some(s) = lwith_int(b, state, "moisture", moisture - 1)
                {
                    set(level, ticks, pos, s, 2);
                }
            } else if moisture < 7 {
                if let Some(s) = lwith_int(b, state, "moisture", 7) {
                    set(level, ticks, pos, s, 2);
                }
            } else {
                rich_boost(level, ticks, pos, true);
            }
            true
        }
        // OrganicCompostBlock.randomTick
        "organic_compost" => {
            let mut chance = 0.0f32;
            let mut water = false;
            let mut max_light = 0;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let p = off(pos, dx, dy, dz);
                        let s = level.block_state(p);
                        if in_block_tag(level, "farmersdelight:compost_activators", s) {
                            chance += 0.02;
                        }
                        if b.fluid(s).kind().same_family(horizon_world::fluid::FluidKind::Water) {
                            water = true;
                        }
                        max_light = max_light.max(level.sky_light(off(p, 0, 1, 0)));
                    }
                }
            }
            chance += if max_light > 12 { 0.1 } else { 0.05 };
            chance += if water { 0.1 } else { 0.0 };
            if level.random().next_float() <= chance {
                let stage = lprop_int(b, state, "composting");
                let next = if stage == 7 { ldefault(b, "farmersdelight:rich_soil") } else { lwith_int(b, state, "composting", stage + 1) };
                if let Some(s) = next {
                    set(level, ticks, pos, s, 3);
                }
            }
            true
        }
        // BuddingBushBlock.randomTick, growing into the vine past age 3.
        "budding_tomatoes" => {
            if brightness(level, pos) >= 9 {
                let age = lprop_int(b, state, "age");
                if age <= 3 {
                    let speed = growth_speed(level, pos, &name);
                    if lnext_int(level, (25.0 / speed) as i32 + 1) == 0 {
                        let next = if age >= 3 { ldefault(b, "farmersdelight:tomatoes") } else { lwith_int(b, state, "age", age + 1) };
                        if let Some(s) = next {
                            set(level, ticks, pos, s, 3);
                        }
                    }
                }
            }
            true
        }
        // TomatoVineBlock.randomTick
        "tomatoes" => {
            if brightness(level, pos) >= 9 {
                let age = lprop_int(b, state, "age");
                if age < 3 {
                    let speed = growth_speed(level, pos, &name);
                    if lnext_int(level, (25.0 / speed) as i32 + 1) == 0
                        && let Some(s) = lwith_int(b, state, "age", age + 1)
                    {
                        set(level, ticks, pos, s, 2);
                    }
                }
                rope_climb(level, ticks, pos);
            }
            true
        }
        // RiceBlock.tick (its random tick), growing panicles past age 3.
        "rice" => {
            let above = off(pos, 0, 1, 0);
            let supporting = lname(b, level.block_state(above)) == "farmersdelight:rice_panicles";
            if lprop_bool(b, state, "supporting") != supporting
                && let Some(s) = lwith_bool(b, state, "supporting", supporting)
            {
                set(level, ticks, pos, s, 2);
                return true;
            }
            if brightness(level, above) >= 6 {
                let age = lprop_int(b, state, "age");
                if age <= 3 && lnext_int(level, 3) == 0 {
                    if age == 3 {
                        let free = b.is_air(level.block_state(above));
                        if free
                            && (brightness(level, above) >= 8 || sees_sky(level, above))
                            && let Some(p) = ldefault(b, "farmersdelight:rice_panicles")
                        {
                            set(level, ticks, above, p, 3);
                            if let Some(s) = lwith_bool(b, state, "supporting", true) {
                                set(level, ticks, pos, s, 2);
                            }
                        }
                    } else if let Some(s) = lwith_int(b, state, "age", age + 1) {
                        set(level, ticks, pos, s, 2);
                    }
                }
            }
            true
        }
        // RicePaniclesBlock (CropBlock.randomTick)
        "rice_panicles" => {
            if brightness(level, pos) >= 9 {
                let age = lprop_int(b, state, "age");
                if age < 3 {
                    let speed = growth_speed(level, pos, &name);
                    if lnext_int(level, (25.0 / speed) as i32 + 1) == 0
                        && let Some(s) = lwith_int(b, state, "age", age + 1)
                    {
                        set(level, ticks, pos, s, 2);
                    }
                }
            }
            true
        }
        // MushroomColonyBlock.tick (its random tick)
        "brown_mushroom_colony" | "red_mushroom_colony" => {
            let age = lprop_int(b, state, "age");
            let ground = level.block_state(off(pos, 0, -1, 0));
            if age < 3
                && in_block_tag(level, "farmersdelight:mushroom_colony_growable_on", ground)
                && lnext_int(level, 4) == 0
                && let Some(s) = lwith_int(b, state, "age", age + 1)
            {
                set(level, ticks, pos, s, 2);
            }
            true
        }
        _ => false,
    }
}

/// `TomatoVineBlock.attemptRopeClimb`: a vine under a rope climbs it, up to 3 high.
fn rope_climb(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos) {
    if level.random().next_float() >= 0.3 {
        return;
    }
    let b = level.behaviours;
    let above = off(pos, 0, 1, 0);
    if !in_block_tag(level, "farmersdelight:ropes", level.block_state(above)) {
        return;
    }
    let mut height = 1;
    while lname(b, level.block_state(off(pos, 0, -height, 0))) == "farmersdelight:tomatoes" {
        height += 1;
    }
    if height < 3
        && let Some(s) = ldefault(b, "farmersdelight:tomatoes").and_then(|d| lwith_bool(b, d, "ropelogged", true))
    {
        set(level, ticks, above, s, 3);
    }
}

/// `RichSoilFarmlandBlock.tick`: buried farmland turns back into rich soil, not dirt.
pub(crate) fn tick_block(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) -> bool {
    let b = level.behaviours;
    if lname(b, state) != "farmersdelight:rich_soil_farmland" {
        return false;
    }
    let above = level.block_state(off(pos, 0, 1, 0));
    if b.is_solid(above) && !in_block_tag(level, "minecraft:fence_gates", above) {
        if let Some(s) = ldefault(b, "farmersdelight:rich_soil") {
            set(level, ticks, pos, s, 3);
        }
    }
    true
}

/// `isValidBonemealTarget`.
pub(crate) fn bonemeal_valid(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId) -> Option<bool> {
    let b = level.behaviours;
    let n = short(lname(b, state))?;
    Some(match n {
        "budding_tomatoes" => true,
        "tomatoes" | "rice_panicles" | "brown_mushroom_colony" | "red_mushroom_colony" => lprop_int(b, state, "age") < 3,
        "rice" => {
            let up = level.block_state(off(pos, 0, 1, 0));
            lname(b, up) != "farmersdelight:rice_panicles" || lprop_int(b, up, "age") < 3
        }
        _ => return None,
    })
}

/// `performBonemeal`.
pub(crate) fn bonemeal_perform(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    let b = level.behaviours;
    let Some(n) = short(lname(b, state)).map(str::to_owned) else { return };
    let age = lprop_int(b, state, "age");
    match n.as_str() {
        // BuddingTomatoBlock.performBonemeal
        "budding_tomatoes" => {
            let grown = (age + lbetween(level, 1, 4)).min(7);
            let next = if grown <= 3 {
                lwith_int(b, state, "age", grown)
            } else {
                ldefault(b, "farmersdelight:tomatoes").and_then(|d| lwith_int(b, d, "age", grown - 4))
            };
            if let Some(s) = next {
                set(level, ticks, pos, s, 3);
            }
        }
        // TomatoVineBlock.performBonemeal
        "tomatoes" => {
            let grown = (age + lbetween(level, 2, 5) / 2).min(3);
            if let Some(s) = lwith_int(b, state, "age", grown) {
                set(level, ticks, pos, s, 3);
            }
            rope_climb(level, ticks, pos);
        }
        // RicePaniclesBlock: CropBlock's increase over 3
        "rice_panicles" => {
            let grown = (age + lbetween(level, 2, 5) / 3).min(3);
            if let Some(s) = lwith_int(b, state, "age", grown) {
                set(level, ticks, pos, s, 3);
            }
        }
        "brown_mushroom_colony" | "red_mushroom_colony" => {
            let grown = (age + lbetween(level, 1, 2)).min(3);
            if let Some(s) = lwith_int(b, state, "age", grown) {
                set(level, ticks, pos, s, 2);
            }
        }
        // RiceBlock.performBonemeal
        "rice" => {
            let grown = (age + lbetween(level, 1, 4)).min(7);
            if grown <= 3 {
                if let Some(s) = lwith_int(b, state, "age", grown) {
                    set(level, ticks, pos, s, 3);
                }
                return;
            }
            let above = off(pos, 0, 1, 0);
            let up = level.block_state(above);
            if lname(b, up) == "farmersdelight:rice_panicles" {
                if lprop_int(b, up, "age") < 3 {
                    bonemeal_perform(level, ticks, above, up);
                }
            } else if b.is_air(up)
                && (brightness(level, above) >= 8 || sees_sky(level, above))
                && let Some(p) = ldefault(b, "farmersdelight:rice_panicles").and_then(|d| lwith_int(b, d, "age", grown - 4))
            {
                if let Some(s) = lwith_int(b, state, "age", 3).and_then(|s| lwith_bool(b, s, "supporting", true)) {
                    set(level, ticks, pos, s, 3);
                }
                set(level, ticks, above, p, 2);
            }
        }
        _ => {}
    }
}

/// `canSurvive` where the template's rule isn't Farmer's Delight's.
pub(crate) fn can_survive(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId) -> Option<bool> {
    let b = level.behaviours;
    let n = short(lname(b, state))?;
    let below = level.block_state(off(pos, 0, -1, 0));
    let light = brightness(level, pos) >= 8 || sees_sky(level, pos);
    match n {
        "tomatoes" if lprop_bool(b, state, "ropelogged") => Some(lname(b, below) == "farmersdelight:tomatoes" && light),
        "rice_panicles" => Some(lname(b, below) == "farmersdelight:rice" && light),
        "brown_mushroom_colony" | "red_mushroom_colony" => {
            let g = lname(b, below);
            Some(
                in_block_tag(level, "minecraft:mushroom_grow_block", below)
                    || g == "farmersdelight:rich_soil"
                    || g == "farmersdelight:rich_soil_farmland"
                    || (brightness(level, pos) < 13 && b.is_solid(below)),
            )
        }
        _ => None,
    }
}

/// `mayPlaceOn` for the plants on a template whose ground isn't theirs.
pub(crate) fn may_place_on(level: &GameLevel<'_>, state: BlockStateId, ground: BlockStateId) -> Option<bool> {
    let b = level.behaviours;
    match short(lname(b, state))? {
        "rice_panicles" => Some(lname(b, ground) == "farmersdelight:rice"),
        "brown_mushroom_colony" | "red_mushroom_colony" => Some(b.is_solid(ground)),
        "tomatoes" | "budding_tomatoes" => {
            let g = lname(b, ground);
            Some(g == "minecraft:farmland" || g == "farmersdelight:rich_soil_farmland" || (g == "farmersdelight:tomatoes"))
        }
        _ => None,
    }
}

/// `getStateForPlacement`: `Some(Some(state))` to choose one, `Some(None)` to refuse.
#[allow(clippy::option_option)]
pub(crate) fn placement(
    level: &GameLevel<'_>,
    state: BlockStateId,
    context: &PlacementContext,
    fluid: FluidState,
) -> Option<Option<BlockStateId>> {
    let b = level.behaviours;
    let n = short(lname(b, state))?;
    let hor = horizontal(context.yaw);
    let tray = || in_block_tag(level, "farmersdelight:tray_heat_sources", level.block_state(off(context.pos, 0, -1, 0)));
    let face = dir_name(context.face);
    let facing = |s: BlockStateId, d: &str| lwith_text(b, s, "facing", d);
    Some(match n {
        "stove" => facing(state, opposite(hor)).and_then(|s| lwith_bool(b, s, "lit", true)),
        "cooking_pot" => {
            let support = if face == "down" { "handle" } else if tray() { "tray" } else { "none" };
            facing(state, opposite(hor)).and_then(|s| lwith_text(b, s, "support", support))
        }
        "skillet" => facing(state, hor).and_then(|s| lwith_bool(b, s, "support", tray())),
        "cutting_board" => facing(state, opposite(hor)),
        "rice_bale" => facing(state, face),
        "tatami" => {
            let ts = level.block_state(context.hit_pos);
            let pairing = !context.sneaking && lname(b, ts) == "farmersdelight:tatami" && !lprop_bool(b, ts, "paired");
            facing(state, opposite(face)).and_then(|s| lwith_bool(b, s, "paired", pairing))
        }
        "full_tatami_mat" => {
            let (dx, dz) = step(hor);
            let pair = level.block_state(off(context.pos, dx, 0, dz));
            if !b.is_air(pair) && !level.facts.get(pair).replaceable {
                return Some(None);
            }
            facing(state, hor).and_then(|s| lwith_text(b, s, "part", "foot"))
        }
        "half_tatami_mat" => facing(state, hor),
        "rice" => {
            if !(fluid.kind() == horizon_world::fluid::FluidKind::Water && fluid.is_full()) {
                return Some(None);
            }
            Some(state)
        }
        "brown_mushroom_colony" | "red_mushroom_colony" => lwith_int(b, state, "age", 3),
        _ if n.ends_with("_cabinet") => facing(state, opposite(hor)),
        _ if is_pie(n) => facing(state, hor),
        _ if feast(n).is_some() => facing(state, opposite(hor)),
        _ => return None,
    })
}
