// Easy Magic 8.0.1 (Fuzs), the server's side: the re-roll (ModEnchantmentMenu.clickMenuButton
// with id 4) and the table that keeps its item and lapis (ModEnchantmentTableBlockEntity's
// inventory) between uses.
//
// Injected as `crate::game::lan_stations::easymagic`.
use super::*;
use horizon_world::attachments::{AttachmentCodec, AttachmentError, AttachmentType};
use horizon_world::nbt::{Compound, Tag};

const REROLL: i32 = 4;
const LAPIS_COST: u32 = 1;
const XP_COST: i32 = 5;

/// The level and bar left after paying `points` (Player.giveExperiencePoints with a negative
/// amount, which can go down levels).
fn pay_points(level: i32, progress: f32, points: i32) -> (i32, f32) {
    let needed = horizon_client::entities::Experience::needed;
    let mut level = level;
    let mut have = (progress * needed(level) as f32).round() as i32 - points;
    while have < 0 && level > 0 {
        level -= 1;
        have += needed(level);
    }
    let have = have.max(0);
    (level, have as f32 / needed(level) as f32)
}
fn total_xp(level: i32, progress: f32) -> i32 {
    let levels = if level < 17 {
        level * level + 6 * level
    } else if level < 32 {
        (2.5 * (level * level) as f32 - 40.5 * level as f32 + 360.0) as i32
    } else {
        (4.5 * (level * level) as f32 - 162.5 * level as f32 + 2220.0) as i32
    };
    levels + (horizon_client::entities::Experience::needed(level) as f32 * progress) as i32
}

/// clickMenuButton(4): with an item that has offers, and the lapis and points (or creative), the
/// player gets a new enchantment seed, pays, and the offers are made again. True when `id` was
/// the re-roll.
pub(crate) fn reroll(game: &mut Game<'_>, host: &mut LanHost, conn: u32, window: i32, id: i32) -> bool {
    if id != REROLL {
        return false;
    }
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
        return true;
    };
    let Some(mut gs) = host
        .play
        .get_mut(&conn)
        .and_then(|p| p.station.take_if(|s| s.window == window && s.kind == StationKind::Enchanting))
    else {
        return true;
    };
    let g = &host.hub.guests[gi];
    let creative = g.creative();
    let lapis = gs.menu.slot(1).map_or(0, |l| l.count);
    let usable = gs.menu.slot(0).is_some() && gs.menu.data[0..3].iter().any(|c| *c > 0);
    let afford = creative || lapis >= LAPIS_COST && total_xp(g.xp_level, g.xp_progress) >= XP_COST;
    if usable && afford && !g.spectator() {
        host.hub.guests[gi].xp_seed = game.world.random.next_int();
        if !creative {
            let env = game.menu_env();
            let left = gs.menu.slot(1).cloned().map(|mut l| {
                l.count -= LAPIS_COST;
                l
            });
            gs.menu.set_slot(&env, 1, left.filter(|l| l.count > 0));
            game.as_guest(host, conn, |game| {
                let (level, progress) =
                    pay_points(game.experience.level, game.experience.progress, XP_COST);
                let old = game.experience.level;
                game.experience.level = level;
                game.experience.progress = progress;
                game.experience.total = (game.experience.total - XP_COST).max(0);
                if level != old {
                    game.native_player_level_changed(old);
                }
            });
        }
        gs.menu.enchant_changed = false;
        game.lan_enchant_offers(host, gi, &mut gs);
    }
    if let Some(p) = host.play.get_mut(&conn) {
        p.station = Some(gs);
    }
    game.lan_send_station(host, conn);
    true
}

/// What each table holds between uses, by position (ModEnchantmentTableBlockEntity's two
/// slots), saved with the level.
#[derive(Clone, Default)]
pub(crate) struct EmTables(Vec<([i32; 3], [Option<ItemStack>; 2])>);
fn encode(t: &EmTables) -> std::result::Result<Compound, AttachmentError> {
    let list = t
        .0
        .iter()
        .map(|(pos, slots)| {
            let mut c = Compound::new();
            c.insert("pos".into(), Tag::IntArray(pos.to_vec()));
            for (i, s) in slots.iter().enumerate() {
                if let Some(s) = s {
                    c.insert(format!("slot{i}").as_str().into(), horizon_client::save::item_to_nbt(s));
                }
            }
            Tag::Compound(c)
        })
        .collect();
    let mut c = Compound::new();
    c.insert("tables".into(), Tag::List(list));
    Ok(c)
}
fn decode(c: &Compound) -> std::result::Result<EmTables, AttachmentError> {
    let mut out = Vec::new();
    if let Some(Tag::List(list)) = c.get(&"tables".into()) {
        for t in list {
            let Tag::Compound(t) = t else { continue };
            let Some(Tag::IntArray(p)) = t.get(&"pos".into()) else { continue };
            if p.len() != 3 {
                continue;
            }
            let slot = |i: usize| {
                t.get(&format!("slot{i}").as_str().into())
                    .and_then(horizon_client::save::item_from_nbt)
            };
            out.push(([p[0], p[1], p[2]], [slot(0), slot(1)]));
        }
    }
    Ok(EmTables(out))
}
fn none() -> EmTables {
    EmTables::default()
}
static TABLES: AttachmentType<EmTables> = AttachmentType {
    namespace: "easymagic",
    name: "tables",
    default: none,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// AbstractContainerMenu.removed for the table's menu: the item and the lapis stay in the table
/// instead of going back to the player (only the carried stack does).
pub(crate) fn keep(game: &mut Game<'_>, host: &mut LanHost, conn: u32, window: Option<i32>) {
    let Some(gs) = host.play.get_mut(&conn).and_then(|p| p.station.as_mut()) else {
        return;
    };
    if gs.kind != StationKind::Enchanting || window.is_some_and(|w| w != gs.window) {
        return;
    }
    let slots = [gs.menu.slot(0).cloned(), gs.menu.slot(1).cloned()];
    if slots.iter().all(Option::is_none) {
        return;
    }
    let env = game.menu_env();
    gs.menu.set_slot(&env, 0, None);
    gs.menu.set_slot(&env, 1, None);
    let pos = gs.pos;
    let mut tables = game.world.attachments.get(&TABLES).unwrap_or_default();
    // Something already kept there (two players at one table) goes in first, as far as it fits.
    if let Some(i) = tables.0.iter().position(|(p, _)| *p == pos) {
        let (_, old) = tables.0.remove(i);
        for s in old.into_iter().flatten() {
            game.pop_resource(pos, s);
        }
    }
    tables.0.push((pos, slots));
    let _ = game.world.attachments.set(&TABLES, tables);
}

/// The first time the guest's table menu is sent: what the table keeps goes into its slots, and
/// the offers are made for it.
pub(crate) fn restore(game: &mut Game<'_>, host: &mut LanHost, conn: u32) {
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
        return;
    };
    let fresh = host.play.get(&conn).and_then(|p| p.station.as_ref()).is_some_and(|s| {
        s.kind == StationKind::Enchanting && s.state == 0 && s.sent_data.is_empty()
    });
    if !fresh || !game.world.attachments.has(&TABLES) {
        return;
    }
    let Some(mut gs) = host.play.get_mut(&conn).and_then(|p| p.station.take()) else {
        return;
    };
    let mut tables = game.world.attachments.get(&TABLES).unwrap_or_default();
    if let Some(i) = tables.0.iter().position(|(p, _)| *p == gs.pos) {
        let (_, [item, lapis]) = tables.0.remove(i);
        let _ = game.world.attachments.set(&TABLES, tables);
        let env = game.menu_env();
        gs.menu.set_slot(&env, 0, item);
        gs.menu.set_slot(&env, 1, lapis);
        gs.menu.enchant_changed = false;
        game.lan_enchant_offers(host, gi, &mut gs);
    }
    if let Some(p) = host.play.get_mut(&conn) {
        p.station = Some(gs);
    }
}

/// Once a tick: a table that is gone (broken, blown up) drops what it kept
/// (Containers.dropContents in onRemove).
pub(crate) fn watch(game: &mut Game<'_>) {
    static LAST: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);
    let now = game.world.game_time;
    if LAST.swap(now, std::sync::atomic::Ordering::Relaxed) == now
        || !game.world.attachments.has(&TABLES)
    {
        return;
    }
    let mut tables = game.world.attachments.get(&TABLES).unwrap_or_default();
    let mut gone = Vec::new();
    tables.0.retain(|(p, slots)| {
        let loaded = game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4));
        let table = &*game.res.facts().get(game.world.block(p[0], p[1], p[2])).name
            == "minecraft:enchanting_table";
        if loaded && !table {
            gone.push((*p, slots.clone()));
            false
        } else {
            true
        }
    });
    if gone.is_empty() {
        return;
    }
    let _ = game.world.attachments.set(&TABLES, tables);
    for (p, slots) in gone {
        for s in slots.into_iter().flatten() {
            game.pop_resource(p, s);
        }
    }
}
