// Universal Graves 3.0.3 (Patbox), lite: a player's death puts their items and dropped
// experience into a grave (a skeleton skull) instead of on the ground. A child of `game`, so
// the Game's own fields and helpers (inventory, armor, death_drops, spawn_orbs) are in reach.
use super::*;
use horizon_world::attachments::{AttachmentCodec, AttachmentError, AttachmentType};
use horizon_world::nbt::{Compound, Tag};

/// protection.non_owner_protection_time: 900 s of game time (use_real_time false).
const PROTECTION_TICKS: i64 = 900 * 20;
/// placement.max_distance_from_source_location.
const MAX_DISTANCE: i32 = 8;
const GRAVE_BLOCK: &str = "minecraft:skeleton_skull";
/// Slot codes: 0..36 the inventory, 100 + i armor[i] (0 is the head), 200 the off hand. Trinkets
/// (Universal Graves' Trinkets compatibility) are kept apart, by their slot id.
const ARMOR: u8 = 100;
const OFFHAND: u8 = 200;

#[derive(Clone)]
pub(crate) struct UgGrave {
    pos: [i32; 3],
    owner: u128,
    name: String,
    created: i64,
    xp: u32,
    items: Vec<(u8, ItemStack)>,
    /// The worn trinkets (native/trinkets' `trinkets:worn`), by slot id.
    trinkets: Vec<(String, ItemStack)>,
}
#[derive(Clone, Default)]
pub(crate) struct UgGraves(Vec<UgGrave>);

fn encode(g: &UgGraves) -> std::result::Result<Compound, AttachmentError> {
    let list = g
        .0
        .iter()
        .map(|g| {
            let items = g
                .items
                .iter()
                .map(|(slot, stack)| {
                    let mut t = horizon_client::save::item_to_nbt(stack);
                    if let Tag::Compound(c) = &mut t {
                        c.insert("ug_slot".into(), Tag::Byte(*slot as i8));
                    }
                    t
                })
                .chain(g.trinkets.iter().map(|(id, stack)| {
                    let mut t = horizon_client::save::item_to_nbt(stack);
                    if let Tag::Compound(c) = &mut t {
                        c.insert("ug_trinket".into(), Tag::String(id.as_str().into()));
                    }
                    t
                }))
                .collect();
            let mut c = Compound::new();
            c.insert("pos".into(), Tag::IntArray(g.pos.to_vec()));
            c.insert(
                "owner".into(),
                Tag::LongArray(vec![(g.owner >> 64) as i64, g.owner as u64 as i64]),
            );
            c.insert("name".into(), Tag::String(g.name.as_str().into()));
            c.insert("created".into(), Tag::Long(g.created));
            c.insert("xp".into(), Tag::Int(g.xp as i32));
            c.insert("items".into(), Tag::List(items));
            Tag::Compound(c)
        })
        .collect();
    let mut c = Compound::new();
    c.insert("graves".into(), Tag::List(list));
    Ok(c)
}
fn decode(c: &Compound) -> std::result::Result<UgGraves, AttachmentError> {
    let mut out = Vec::new();
    let Some(Tag::List(list)) = c.get(&"graves".into()) else {
        return Ok(UgGraves::default());
    };
    for t in list {
        let Tag::Compound(g) = t else { continue };
        let Some(Tag::IntArray(pos)) = g.get(&"pos".into()) else { continue };
        if pos.len() != 3 {
            continue;
        }
        let owner = match g.get(&"owner".into()) {
            Some(Tag::LongArray(v)) if v.len() == 2 => {
                (u128::from(v[0] as u64) << 64) | u128::from(v[1] as u64)
            }
            _ => continue,
        };
        let name = match g.get(&"name".into()) {
            Some(Tag::String(s)) => String::from_utf16_lossy(&s.0),
            _ => String::new(),
        };
        let created = match g.get(&"created".into()) {
            Some(Tag::Long(v)) => *v,
            _ => 0,
        };
        let xp = match g.get(&"xp".into()) {
            Some(Tag::Int(v)) => *v as u32,
            _ => 0,
        };
        let mut items = Vec::new();
        let mut trinkets = Vec::new();
        if let Some(Tag::List(list)) = g.get(&"items".into()) {
            for t in list {
                if let Tag::Compound(c) = t
                    && let Some(Tag::String(id)) = c.get(&"ug_trinket".into())
                {
                    if let Some(stack) = horizon_client::save::item_from_nbt(t) {
                        trinkets.push((String::from_utf16_lossy(&id.0), stack));
                    }
                    continue;
                }
                let slot = match t {
                    Tag::Compound(c) => match c.get(&"ug_slot".into()) {
                        Some(Tag::Byte(b)) => *b as u8,
                        _ => 0,
                    },
                    _ => continue,
                };
                if let Some(stack) = horizon_client::save::item_from_nbt(t) {
                    items.push((slot, stack));
                }
            }
        }
        out.push(UgGrave {
            pos: [pos[0], pos[1], pos[2]],
            owner,
            name,
            created,
            xp,
            items,
            trinkets,
        });
    }
    Ok(UgGraves(out))
}
fn none() -> UgGraves {
    UgGraves::default()
}
static GRAVES: AttachmentType<UgGraves> = AttachmentType {
    namespace: "universal_graves",
    name: "graves",
    default: none,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

fn same(c: &Compound) -> std::result::Result<Compound, AttachmentError> {
    Ok(c.clone())
}
fn no_trinkets() -> Compound {
    Compound::new()
}
/// native/trinkets' worn trinkets (slot id -> item NBT). Declared here with the same key and type,
/// so it reads the same value; without native/trinkets nothing is ever worn.
static TRINKETS: AttachmentType<Compound> = AttachmentType {
    namespace: "trinkets",
    name: "worn",
    default: no_trinkets,
    codec: Some(AttachmentCodec { encode: same, decode: same }),
    copy_on_death: true,
    sync: true,
};
fn worn_trinkets(game: &mut Game<'_>, uuid: u128) -> Vec<(String, ItemStack)> {
    let host: &mut dyn horizon_client::mod_api::Host = game;
    let c = TRINKETS.get(host, horizon_world::attachments::Holder::Player(uuid)).unwrap_or_default();
    c.iter()
        .filter_map(|(k, t)| horizon_client::save::item_from_nbt(t).map(|s| (String::from_utf16_lossy(&k.0), s)))
        .collect()
}
fn set_trinkets(game: &mut Game<'_>, uuid: u128, worn: &[(String, ItemStack)]) -> bool {
    let mut c = Compound::new();
    for (id, s) in worn {
        c.insert(id.as_str().into(), horizon_client::save::item_to_nbt(s));
    }
    let host: &mut dyn horizon_client::mod_api::Host = game;
    TRINKETS.set(host, horizon_world::attachments::Holder::Player(uuid), c).is_ok()
}

fn load(game: &mut Game<'_>) -> UgGraves {
    game.world.attachments.get(&GRAVES).unwrap_or_default()
}
fn store(game: &mut Game<'_>, graves: UgGraves) {
    let _ = game.world.attachments.set(&GRAVES, graves);
}

/// Messages for players, sent from their next tick (the death and the click run in the
/// player's turn, where chat goes to the server's own log).
static OUTBOX: std::sync::Mutex<Vec<(u128, String)>> = std::sync::Mutex::new(Vec::new());
fn tell(player: u128, text: String) {
    OUTBOX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push((player, text));
}
pub(crate) fn take_messages(player: u128) -> Vec<String> {
    let mut out = OUTBOX.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut mine = Vec::new();
    out.retain(|(p, t)| {
        if *p == player {
            mine.push(t.clone());
            false
        } else {
            true
        }
    });
    mine
}

/// The grave the experience of this death goes to, and the tick: spawn_orbs runs right after
/// the drops, in the same tick (unless a sculk catalyst took the experience).
static XP_TO: std::sync::Mutex<Option<([i32; 3], i64)>> = std::sync::Mutex::new(None);

fn spot_ok(game: &Game<'_>, p: [i32; 3]) -> bool {
    let s = game.world.block(p[0], p[1], p[2]);
    let facts = game.res.facts();
    facts.is_air(s) || matches!(&*facts.get(s).name, "minecraft:short_grass" | "minecraft:fern")
}
/// GraveUtils.findGravePosition, simply: the nearest free (air or grass) block within 8 of
/// where the player died, the death height kept inside the world.
fn find_spot(game: &Game<'_>, feet: [i32; 3]) -> Option<[i32; 3]> {
    let min = game.world.shape.min_y;
    let max = game.world.shape.max_y() - 1;
    let c = [feet[0], feet[1].clamp(min, max), feet[2]];
    let mut best: Option<(i32, [i32; 3])> = None;
    for dx in -MAX_DISTANCE..=MAX_DISTANCE {
        for dy in -MAX_DISTANCE..=MAX_DISTANCE {
            for dz in -MAX_DISTANCE..=MAX_DISTANCE {
                let p = [c[0] + dx, c[1] + dy, c[2] + dz];
                if p[1] < min || p[1] > max {
                    continue;
                }
                // Up is preferred over down at the same distance (out of water and lava).
                let d = dx * dx + dy * dy + dz * dz;
                let key = d * 2 + i32::from(dy < 0);
                if best.is_some_and(|(b, _)| key >= b) {
                    continue;
                }
                if game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4)) && spot_ok(game, p) {
                    best = Some((key, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}
fn skull_state(game: &Game<'_>, yaw: f32) -> Option<BlockStateId> {
    let reg = game.pack.assets.registry;
    let mut st = reg.state(reg.default_state(GRAVE_BLOCK)?)?.clone();
    // RotationSegment.convertToSegment(yRot + 180), as a placed skull faces.
    let r = (((yaw + 180.0) / 22.5).round() as i32).rem_euclid(16);
    st.properties.insert("rotation".into(), r.to_string());
    reg.resolve(&st).ok()
}

/// Player.dropEquipment for a guest in their turn: the stacks go into a new grave with their
/// slots (vanishing-cursed ones are destroyed, as vanilla does). False leaves the drop to
/// vanilla: keep_inventory, nothing to store, or no room for a grave.
pub(crate) fn bury(game: &mut Game<'_>) -> bool {
    if game.ui.rules.bool("keep_inventory") {
        return false;
    }
    let (Some(owner), Some(name)) = (game.lan_acting_uuid, game.lan_acting_name.clone()) else {
        return false;
    };
    let Some(feet) = game.player.as_ref().map(|p| p.position) else {
        return false;
    };
    let feet = [feet.x.floor() as i32, feet.y.floor() as i32, feet.z.floor() as i32];
    let cursed = |s: &ItemStack| s.enchantment_level("minecraft:vanishing_curse") > 0;
    let any = game.inventory.iter().flatten().any(|s| !cursed(s))
        || game.armor.iter().flatten().any(|s| !cursed(s))
        || game.offhand.as_ref().is_some_and(|s| !cursed(s))
        || worn_trinkets(game, owner).iter().any(|(_, s)| !cursed(s));
    if !any {
        return false;
    }
    let Some(pos) = find_spot(game, feet) else { return false };
    let Some(skull) = skull_state(game, game.yaw) else { return false };
    let mut items = Vec::new();
    for (i, s) in game.inventory.iter_mut().enumerate() {
        if let Some(s) = s.take() {
            items.push((i as u8, s));
        }
    }
    for (i, s) in game.armor.iter_mut().enumerate() {
        if let Some(s) = s.take() {
            items.push((ARMOR + i as u8, s));
        }
    }
    if let Some(s) = game.offhand.take() {
        items.push((OFFHAND, s));
    }
    let mut trinkets = worn_trinkets(game, owner);
    if !trinkets.is_empty() {
        let _ = set_trinkets(game, owner, &[]);
    }
    items.retain(|(_, s)| !cursed(s));
    trinkets.retain(|(_, s)| !cursed(s));
    game.apply_equipment();
    if game.level_set(pos, skull).is_err() {
        // No grave after all: the stacks drop as vanilla's would.
        for s in items.into_iter().map(|(_, s)| s).chain(trinkets.into_iter().map(|(_, s)| s)) {
            game.pop_resource(pos, s);
        }
        return true;
    }
    let count: u32 = items.iter().map(|(_, s)| s.count).chain(trinkets.iter().map(|(_, s)| s.count)).sum();
    let mut graves = load(game);
    graves.0.retain(|g| g.pos != pos);
    graves.0.push(UgGrave {
        pos,
        owner,
        name,
        created: game.world.game_time,
        xp: 0,
        items,
        trinkets,
    });
    store(game, graves);
    *XP_TO.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some((pos, game.world.game_time));
    tell(
        owner,
        format!(
            "Created grave at {} {} {} ({} items). It stays protected from others for 15 minutes.",
            pos[0], pos[1], pos[2], count
        ),
    );
    true
}

/// The death's experience goes into the grave just made (experience_type percent_points 100:
/// all of what vanilla would drop).
pub(crate) fn take_xp(game: &mut Game<'_>, value: u32) -> bool {
    let Some((pos, tick)) = XP_TO.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take()
    else {
        return false;
    };
    if tick != game.world.game_time {
        return false;
    }
    let mut graves = load(game);
    let Some(g) = graves.0.iter_mut().find(|g| g.pos == pos) else {
        return false;
    };
    g.xp += value;
    store(game, graves);
    true
}

fn allowed(game: &Game<'_>, g: &UgGrave) -> bool {
    game.creative
        || game.lan_acting_uuid == Some(g.owner)
        || game.world.game_time - g.created >= PROTECTION_TICKS
}
fn center(p: [i32; 3]) -> [f64; 3] {
    [f64::from(p[0]) + 0.5, f64::from(p[1]) + 0.5, f64::from(p[2]) + 0.5]
}
fn denied(game: &Game<'_>, g: &UgGrave) {
    if let Some(me) = game.lan_acting_uuid {
        let left = (PROTECTION_TICKS - (game.world.game_time - g.created)).max(0) / 20;
        tell(
            me,
            format!(
                "You don't have access to this grave! (Grave of {}, protected for {}:{:02})",
                g.name,
                left / 60,
                left % 60
            ),
        );
    }
}
fn take_grave(game: &mut Game<'_>, pos: [i32; 3]) -> Option<UgGrave> {
    let mut graves = load(game);
    let i = graves.0.iter().position(|g| g.pos == pos)?;
    let g = graves.0.remove(i);
    store(game, graves);
    Some(g)
}
fn clear_block(game: &mut Game<'_>, pos: [i32; 3]) {
    if &*game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name == GRAVE_BLOCK
        && let Some(air) = game.pack.assets.registry.default_state("minecraft:air")
    {
        let _ = game.level_set(pos, air);
    }
}

/// A right click on a grave (shift_and_use_quick_pickup, for any click: the grave screen is not
/// ported): its owner, anyone once the protection is over, or a creative player gets the items
/// back into the slots they came from (else anywhere, else dropped) and the experience.
/// True when the click was the grave's.
pub(crate) fn open(game: &mut Game<'_>) -> bool {
    if !matches!(game.guest_use, Some(GuestUse::OnBlock(_))) {
        return false;
    }
    let Some(hit) = game.pick() else { return false };
    let graves = load(game);
    let Some(g) = graves.0.iter().find(|g| g.pos == hit.pos).cloned() else {
        return false;
    };
    if !allowed(game, &g) {
        denied(game, &g);
        return true;
    }
    let Some(g) = take_grave(game, hit.pos) else { return true };
    clear_block(game, g.pos);
    let mut rest = Vec::new();
    // The trinkets back into the opener's slots, where still empty; else with the rest.
    if let Some(who) = game.lan_acting_uuid.filter(|_| !g.trinkets.is_empty()) {
        let before = worn_trinkets(game, who);
        let mut worn = before.clone();
        let mut placed = Vec::new();
        for (id, s) in g.trinkets.clone() {
            if worn.iter().any(|(w, _)| *w == id) {
                rest.push(s);
            } else {
                worn.push((id, s.clone()));
                placed.push(s);
            }
        }
        if !set_trinkets(game, who, &worn) {
            rest.extend(placed);
        }
    } else {
        rest.extend(g.trinkets.iter().map(|(_, s)| s.clone()));
    }
    for (slot, s) in g.items {
        let free = match slot {
            OFFHAND => game.offhand.is_none().then_some(&mut game.offhand),
            a if a >= ARMOR => game
                .armor
                .get_mut(usize::from(a - ARMOR))
                .filter(|x| x.is_none()),
            i => game.inventory.get_mut(usize::from(i)).filter(|x| x.is_none()),
        };
        match free {
            Some(place) => *place = Some(s),
            None => rest.push(s),
        }
    }
    for mut s in rest {
        let max = game.res.items().get(&s.item).map_or(64, |i| i.max_stack);
        let selected = game.selected;
        if inventory_add(&mut game.inventory, &mut game.offhand, selected, max, &mut s) {
            game.pop_resource(g.pos, s);
        }
    }
    game.apply_equipment();
    if g.xp > 0 {
        let at = game
            .player
            .as_ref()
            .map_or(center(g.pos), |p| [p.position.x, p.position.y, p.position.z]);
        game.spawn_orbs(at, g.xp);
    }
    true
}

/// Breaking a grave: allowed as opening is, it drops everything there; else the grave stays.
/// True when the block was a grave.
pub(crate) fn broken(game: &mut Game<'_>, pos: [i32; 3]) -> bool {
    let graves = load(game);
    let Some(g) = graves.0.iter().find(|g| g.pos == pos).cloned() else {
        return false;
    };
    if !allowed(game, &g) {
        denied(game, &g);
        return true;
    }
    let Some(g) = take_grave(game, pos) else { return true };
    clear_block(game, pos);
    spill(game, g);
    true
}
fn spill(game: &mut Game<'_>, g: UgGrave) {
    for s in g.items.into_iter().map(|(_, s)| s).chain(g.trinkets.into_iter().map(|(_, s)| s)) {
        game.pop_resource(g.pos, s);
    }
    if g.xp > 0 {
        game.spawn_orbs(center(g.pos), g.xp);
    }
}

/// Once a tick: a grave whose skull is gone (an explosion, a piston, a command) drops what it
/// held there, as the grave block's removal does in the original, and its owner hears of it.
pub(crate) fn watch(game: &mut Game<'_>) {
    static LAST: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);
    let now = game.world.game_time;
    if LAST.swap(now, std::sync::atomic::Ordering::Relaxed) == now {
        return;
    }
    if !game.world.attachments.has(&GRAVES) {
        return;
    }
    let graves = load(game);
    let gone: Vec<[i32; 3]> = graves
        .0
        .iter()
        .filter(|g| game.world.chunks.contains_key(&(g.pos[0] >> 4, g.pos[2] >> 4)))
        .filter(|g| {
            &*game.res.facts().get(game.world.block(g.pos[0], g.pos[1], g.pos[2])).name != GRAVE_BLOCK
        })
        .map(|g| g.pos)
        .collect();
    for pos in gone {
        if let Some(g) = take_grave(game, pos) {
            let count: u32 = g.items.iter().map(|(_, s)| s.count).chain(g.trinkets.iter().map(|(_, s)| s.count)).sum();
            tell(
                g.owner,
                format!(
                    "Your grave at {} {} {} containing {} item(s) has been broken!",
                    pos[0], pos[1], pos[2], count
                ),
            );
            spill(game, g);
        }
    }
}
