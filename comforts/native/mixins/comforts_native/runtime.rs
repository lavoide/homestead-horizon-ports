// Comforts 6.4.0 (TheIllusiveC4), the rules of its sleeping bags and hammocks, with Homestead's
// config (comforts-server.toml): autoUse true, sleepingBagUse NIGHT, hammockUse DAY,
// restrictSleeping false, both stop phantoms, break chance 0, no effects, no wake offsets.
// The blocks themselves (bed templates, models, recipes) are the content mod mods/comforts.
//
// Injected as `crate::game::comforts`.
use super::*;
use super::lan::LanHost;

const BAG: &str = "comforts:sleeping_bag_";
const HAMMOCK: &str = "comforts:hammock_";
const ROPE: &str = "comforts:rope_and_nail";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Bag,
    Hammock,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Bag => "sleeping_bag",
            Kind::Hammock => "hammock",
        }
    }
}

pub(crate) fn kind_of(game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> Option<Kind> {
    let name = &*game.res.facts().get(state).name;
    if name.starts_with(BAG) {
        Some(Kind::Bag)
    } else if name.starts_with(HAMMOCK) {
        Some(Kind::Hammock)
    } else {
        None
    }
}
fn kind_at(game: &Game<'_>, p: [i32; 3]) -> Option<Kind> {
    kind_of(game, game.world.block(p[0], p[1], p[2]))
}
fn name_at(game: &Game<'_>, p: [i32; 3]) -> std::sync::Arc<str> {
    game.res.facts().get(game.world.block(p[0], p[1], p[2])).name.clone()
}
fn prop(game: &Game<'_>, p: [i32; 3], key: &str) -> Option<String> {
    game.pack
        .assets
        .registry
        .state(game.world.block(p[0], p[1], p[2]))
        .and_then(|s| s.properties.get(key).cloned())
}
fn with_props(
    game: &Game<'_>,
    state: horizon_world::registry::BlockStateId,
    props: &[(&str, &str)],
) -> Option<horizon_world::registry::BlockStateId> {
    let reg = game.pack.assets.registry;
    let mut st = reg.state(state)?.clone();
    for (k, v) in props {
        st.properties.insert((*k).into(), (*v).into());
    }
    reg.resolve(&st).ok()
}
fn add(p: [i32; 3], d: [i32; 3], n: i32) -> [i32; 3] {
    [p[0] + d[0] * n, p[1] + d[1] * n, p[2] + d[2] * n]
}
fn opposite(facing: &str) -> &'static str {
    match facing {
        "north" => "south",
        "south" => "north",
        "east" => "west",
        _ => "east",
    }
}
fn center(p: [i32; 3]) -> [f64; 3] {
    [f64::from(p[0]) + 0.5, f64::from(p[1]) + 0.5, f64::from(p[2]) + 0.5]
}
fn sound(game: &mut Game<'_>, name: &str, at: [i32; 3]) {
    game.audio.play(name, Some(center(at)), 1.0, 1.0);
}

/// The head and the foot of the bed at `p`, and its facing (from the foot to the head).
fn halves(game: &Game<'_>, p: [i32; 3]) -> Option<([i32; 3], [i32; 3], String)> {
    let facing = prop(game, p, "facing")?;
    let d = facing_offset(&facing);
    if prop(game, p, "part").as_deref() == Some("foot") {
        Some((add(p, d, 1), p, facing))
    } else {
        Some((p, add(p, d, -1), facing))
    }
}

/// ComfortsEvents.checkTime for a hammock (DAY): strictly between 500 and 11500.
fn hammock_hours(game: &Game<'_>) -> bool {
    let t = game.world.day_time.rem_euclid(24000);
    t > 500 && t < 11500
}
/// Level.isDay, which a sleeping bag (NIGHT) leaves to vanilla: thunder darkens it too.
fn is_day(game: &Game<'_>) -> bool {
    !(game.dim == Dim::Overworld && game.sky_darken() >= 4)
}

/// BaseComfortsBlock.trySleep's checks after the bed itself, in its order: a dimension where
/// beds work, the reach, nothing solid on top, the time of day, no monsters within 8 blocks.
/// `Some(None)` is a refusal without a message (NOT_POSSIBLE_HERE).
fn problem(
    game: &Game<'_>,
    kind: Kind,
    head: [i32; 3],
    foot: [i32; 3],
) -> Option<Option<(String, &'static str)>> {
    let position = game.player.as_ref()?.position;
    if game.dim != Dim::Overworld {
        return Some(None);
    }
    let reachable = |bed: [i32; 3]| {
        (position.x - f64::from(bed[0]) - 0.5).abs() <= 3.0
            && (position.y - f64::from(bed[1])).abs() <= 2.0
            && (position.z - f64::from(bed[2]) - 0.5).abs() <= 3.0
    };
    if !reachable(head) && !reachable(foot) {
        let fallback = match kind {
            Kind::Bag => "You may not rest now; the sleeping bag is too far away",
            Kind::Hammock => "You may not rest now; the hammock is too far away",
        };
        return Some(Some((format!("block.comforts.{}.too_far_away", kind.name()), fallback)));
    }
    let blocked = [head, foot].iter().any(|b| {
        let f = game.res.facts().get(game.world.block(b[0], b[1] + 1, b[2]));
        f.full_collision && f.solid_render
    });
    if blocked {
        return Some(Some(("block.minecraft.bed.obstructed".into(), "This bed is obstructed")));
    }
    match kind {
        Kind::Bag if is_day(game) => {
            return Some(Some((
                "block.minecraft.bed.no_sleep".into(),
                "You can sleep only at night or during thunderstorms",
            )));
        }
        Kind::Hammock if !hammock_hours(game) => {
            return Some(Some((
                "block.comforts.hammock.no_sleep".into(),
                "You can only sleep during the day",
            )));
        }
        _ => {}
    }
    let monsters = !game.creative
        && game.mobs.iter().any(|m| {
            horizon_client::mobs::ai::is_monster(m.species())
                && (m.species() != horizon_client::mobs::Species::ZombifiedPiglin
                    || (m.extra.anger > 0 && m.ai.target.is_some()))
                && m.alive()
                && (m.position.x - position.x).abs() < 8.0
                && (m.position.y - position.y).abs() < 5.0
                && (m.position.z - position.z).abs() < 8.0
        });
    if monsters {
        return Some(Some((
            "block.minecraft.bed.not_safe".into(),
            "You may not rest now; there are monsters nearby",
        )));
    }
    None
}

/// BaseComfortsBlock.use and trySleep for a click on a bag or hammock (Game::try_sleep's place,
/// reached through the bed class): as a bed, but the comforts messages, its own hours, and the
/// respawn point never set. True when the block was one of ours.
pub(crate) fn try_sleep(game: &mut Game<'_>, pos: [i32; 3]) -> bool {
    let Some(kind) = kind_at(game, pos) else {
        return false;
    };
    let Some((head, foot, _)) = halves(game, pos) else {
        return true;
    };
    if prop(game, head, "occupied").as_deref() == Some("true") {
        if !game.kick_villager_out_of_bed(head) {
            let fallback = match kind {
                Kind::Bag => "This sleeping bag is occupied",
                Kind::Hammock => "This hammock is occupied",
            };
            game.tell_sleeper(&format!("block.comforts.{}.occupied", kind.name()), fallback, true);
        }
        return true;
    }
    if let Some(p) = problem(game, kind, head, foot) {
        if let Some((key, fallback)) = p {
            game.tell_sleeper(&key, fallback, true);
        }
        return true;
    }
    lie_down(game, head);
    true
}

/// The rest of ServerPlayer.startSleepInBed and LivingEntity.startSleeping, as Game::try_sleep
/// does it (the mods' hooks, the stats, the position), without the respawn point.
fn lie_down(game: &mut Game<'_>, head: [i32; 3]) {
    let mut refused: Option<String> = None;
    if let (Some(player), Some(name)) = (game.lan_acting_uuid, game.lan_acting_name.clone()) {
        game.native_can_player_sleep(player, &name, head, &mut refused);
    }
    if refused.is_some() {
        return;
    }
    if game.ui.rules.int("players_sleeping_percentage") > 100 {
        game.tell_sleeper("sleep.not_possible", "No amount of rest can pass this night", true);
    }
    if let (Some(player), Some(name)) = (game.lan_acting_uuid, game.lan_acting_name.clone()) {
        let mut cancel = false;
        game.native_player_bed_enter(player, &name, head, &mut cancel);
        if cancel {
            return;
        }
    }
    let Some(at) = game.sleeping_position(head) else {
        return;
    };
    game.stop_mining();
    game.left = false;
    game.right = false;
    game.using = None;
    game.sleeping = Some(head);
    game.sleep_timer = 0;
    if let Some(p) = game.player.as_mut() {
        p.position = at;
        p.velocity = Vec3::ZERO;
        p.pitch = 0.0;
        p.teleported = true;
    }
    game.pitch = 0.0;
    if game.lan_acting.is_none() {
        game.previous_position = at;
        game.snap_player_model_pitch();
    }
    game.occupy_bed(head);
    game.award_custom_stat("sleep_in_bed", 1);
    game.trigger_player("minecraft:slept_in_bed");
    // canRest (both stop phantoms): the rest counter starts again, as a bed's does.
    game.stats.reset("custom", "minecraft:time_since_rest");
}

/// A bag placed by a right click, waiting for the next tick to be slept in: the player and the
/// foot position.
static PLACED: std::sync::Mutex<Option<(Option<u128>, [i32; 3])>> = std::sync::Mutex::new(None);
/// Players asleep in a bag they placed that way (autoSleepPos): the head, and whether they
/// were in creative. It goes when they get up.
static AUTO: std::sync::Mutex<Vec<(u128, [i32; 3], bool)>> = std::sync::Mutex::new(Vec::new());
/// Bags to take up on the next tick (after the wake-up has stood the player beside them).
static PICK_UP: std::sync::Mutex<Vec<(String, [i32; 3], bool)>> = std::sync::Mutex::new(Vec::new());

/// Players whose client took a refused bag or hammock out of their hand: the window predicts
/// a block item's placement and spends the stack, so the inventory is sent again.
static RESYNC: std::sync::Mutex<Vec<u128>> = std::sync::Mutex::new(Vec::new());
fn refused(game: &mut Game<'_>) {
    game.use_failed = true;
    if let Some(p) = game.lan_acting_uuid {
        lock(&RESYNC).push(p);
    }
}

fn lock<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The facing a block item takes from the player (getHorizontalDirection).
fn player_facing(game: &Game<'_>) -> &'static str {
    let yaw = game.player.as_ref().map_or(game.yaw, |p| p.yaw);
    match ((yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4) {
        0 => "south",
        1 => "west",
        2 => "north",
        _ => "east",
    }
}

/// ItemStack.useOn for our items, once the block clicked has passed: a hammock only hangs
/// between two ropes (HammockItem.useOn); a bag, unless the player sneaks, is first checked as
/// if slept in (SleepingBagItem.useOn's dry run) and placed only if that passes. True takes the
/// use.
pub(crate) fn use_on(game: &mut Game<'_>, item: &ItemStack, pos: [i32; 3], face: [i32; 3]) -> bool {
    if item.item.starts_with(HAMMOCK) {
        hang(game, item, pos);
        return true;
    }
    if !item.item.starts_with(BAG) || game.keys.contains(&KeyCode::ShiftLeft) {
        return false;
    }
    let clicked = game.world.block(pos[0], pos[1], pos[2]);
    let foot = if game.res.facts().get(clicked).replaceable { pos } else { add(pos, face, 1) };
    let head = add(foot, facing_offset(player_facing(game)), 1);
    if let Some(p) = problem(game, Kind::Bag, head, foot) {
        if let Some((key, fallback)) = p {
            game.tell_sleeper(&key, fallback, true);
        }
        refused(game);
        return true;
    }
    *lock(&PLACED) = Some((game.lan_acting_uuid, foot));
    false
}

/// The tick after a bag was placed by a click: its player lies down in it, in their own turn
/// (SleepingBagItem's autoSleep packet, which the client answers on its next tick).
fn auto_sleep(game: &mut Game<'_>, host: &mut LanHost, gi: usize) {
    let Some(g) = host.hub.guests.get(gi) else { return };
    let (uuid, conn) = (g.uuid, g.conn);
    let foot = {
        let mut placed = lock(&PLACED);
        match *placed {
            Some((Some(who), foot)) if who == uuid => {
                *placed = None;
                foot
            }
            _ => return,
        }
    };
    if kind_at(game, foot) != Some(Kind::Bag) {
        return;
    }
    let Some((head, _, _)) = halves(game, foot) else { return };
    let slept = game.as_guest(host, conn, |game| {
        try_sleep(game, head);
        (game.sleeping == Some(head)).then_some(game.creative)
    });
    if let Some(Some(creative)) = slept {
        let mut auto = lock(&AUTO);
        auto.retain(|(p, _, _)| *p != uuid);
        auto.push((uuid, head, creative));
    }
}

/// ComfortsEvents.onWakeUp: a bag the player put down to sleep in is taken up again.
pub(crate) fn left_bed(game: &Game<'_>, player: u128, head: [i32; 3]) {
    let mut auto = lock(&AUTO);
    if let Some(i) = auto.iter().position(|(p, h, _)| *p == player && *h == head) {
        let (_, head, creative) = auto.remove(i);
        lock(&PICK_UP).push((game.dim.id().to_owned(), head, creative));
    }
}

/// The bag's two halves go (removeBlock), and outside creative the head's drop (the foot's
/// removal breaks the head through updateShape, which drops it).
fn take_up(game: &mut Game<'_>, head: [i32; 3], creative: bool) {
    if kind_at(game, head) != Some(Kind::Bag) {
        return;
    }
    let Some((head, foot, _)) = halves(game, head) else {
        return;
    };
    let item = name_at(game, head);
    let quiet = level_flags::UPDATE_CLIENTS | level_flags::UPDATE_KNOWN_SHAPE;
    let air = game.world.air;
    let _ = game.level_set_with_flags(head, air, quiet);
    if kind_at(game, foot) == Some(Kind::Bag) {
        let _ = game.level_set(foot, air);
    }
    if !creative {
        game.pop_resource(head, ItemStack::new(&*item, 1));
    }
}

/// Once a tick for each guest (Player.tick): the bags waiting to be taken up, and a hammock's
/// sleeper woken once the hours are over (ALLOW_SLEEP_TIME's DENY wakes them).
pub(crate) fn guest_tick(game: &mut Game<'_>, host: &mut LanHost, gi: usize) {
    let dim = game.dim.id().to_owned();
    let due: Vec<_> = {
        let mut list = lock(&PICK_UP);
        let (here, rest): (Vec<_>, Vec<_>) = list.drain(..).partition(|(d, _, _)| *d == dim);
        *list = rest;
        here
    };
    for (_, head, creative) in due {
        take_up(game, head, creative);
    }
    auto_sleep(game, host, gi);
    let Some(g) = host.hub.guests.get(gi) else {
        return;
    };
    let uuid = g.uuid;
    let resync = {
        let mut list = lock(&RESYNC);
        let had = list.contains(&uuid);
        list.retain(|p| *p != uuid);
        had
    };
    if resync {
        host.hub.send_inventory(gi);
    }
    let g = &host.hub.guests[gi];
    if g.dimension == dim
        && let Some(head) = g.bed
        && kind_at(game, head) == Some(Kind::Hammock)
        && !hammock_hours(game)
    {
        game.wake_guest(host, gi);
    }
}

/// ComfortsEvents.getWakeTime: with someone asleep in a hammock and the sun up, everyone
/// wakes at dusk (12001 before the next midnight) instead of the morning.
pub(crate) fn wake_time(game: &Game<'_>, now: i64, wake: &mut i64) {
    let here = game.dim.id();
    let in_hammock = game.sleeping.is_some_and(|h| kind_at(game, h) == Some(Kind::Hammock))
        || game.lan.as_ref().is_some_and(|l| {
            l.hub.guests.iter().any(|g| {
                g.dimension == here && g.bed.is_some_and(|h| kind_at(game, h) == Some(Kind::Hammock))
            })
        });
    if in_hammock && game.sky_darken() < 4 {
        let i = now + 24000;
        *wake = (i - i.rem_euclid(24000) - 12001).max(now);
    }
}

/// Whether `rope` holds nothing any more: `supporting` with no hammock in front of it (the
/// hammock went some way this port doesn't watch, such as an explosion).
fn rope_free(game: &Game<'_>, rope: [i32; 3]) -> bool {
    if prop(game, rope, "supporting").as_deref() != Some("true") {
        return true;
    }
    let Some(facing) = prop(game, rope, "facing") else {
        return true;
    };
    kind_at(game, add(rope, facing_offset(&facing), 1)) != Some(Kind::Hammock)
}
/// HammockItem.hasPartneredRopes: a rope at `other` facing back at `rope`, both free.
fn partnered(game: &Game<'_>, rope: [i32; 3], other: [i32; 3]) -> bool {
    let (Some(a), Some(b)) = (prop(game, rope, "facing"), prop(game, other, "facing")) else {
        return false;
    };
    &*name_at(game, other) == ROPE && b == opposite(&a) && rope_free(game, rope) && rope_free(game, other)
}
fn set_supporting(game: &mut Game<'_>, rope: [i32; 3], on: bool) {
    if &*name_at(game, rope) != ROPE {
        return;
    }
    let state = game.world.block(rope[0], rope[1], rope[2]);
    if let Some(s) = with_props(game, state, &[("supporting", if on { "true" } else { "false" })]) {
        let _ = game.level_set(rope, s);
    }
}

/// HammockItem.useOn: clicked on a rope with a free rope three blocks out facing back, the
/// hammock goes in between (its foot next to the clicked rope), when both blocks are free.
fn hang(game: &mut Game<'_>, item: &ItemStack, rope: [i32; 3]) {
    let refuse = |game: &mut Game<'_>, key: &str, fallback: &str| {
        game.tell_sleeper(key, fallback, true);
        refused(game);
    };
    if &*name_at(game, rope) != ROPE {
        return refuse(game, "block.comforts.hammock.no_rope", "You must hang the hammock on a pair of rope and nail");
    }
    let Some(facing) = prop(game, rope, "facing") else { return };
    let d = facing_offset(&facing);
    let far = add(rope, d, 3);
    if !partnered(game, rope, far) {
        let near = partnered(game, rope, add(rope, d, 1)) || partnered(game, rope, add(rope, d, 2));
        return if near {
            refuse(game, "block.comforts.hammock.no_space", "There is not enough space to fit the hammock")
        } else {
            refuse(
                game,
                "block.comforts.hammock.missing_rope",
                "There must also be a rope and nail on the other end to hang the hammock",
            )
        };
    }
    let (foot, head) = (add(rope, d, 1), add(rope, d, 2));
    let free = |p: [i32; 3]| game.res.facts().get(game.world.block(p[0], p[1], p[2])).replaceable;
    let reg = game.pack.assets.registry;
    let states = reg.default_state(&*item.item).and_then(|s| {
        Some((
            with_props(game, s, &[("facing", facing.as_str()), ("part", "foot")])?,
            with_props(game, s, &[("facing", facing.as_str()), ("part", "head")])?,
        ))
    });
    let (Some((foot_state, head_state)), true) = (states, free(foot) && free(head)) else {
        return refuse(game, "block.comforts.hammock.no_space", "There is not enough space to fit the hammock");
    };
    // BaseComfortsItem.placeBlock (flags 26: no shape update) for the foot, then
    // BedBlock.setPlacedBy's head.
    let quiet = level_flags::UPDATE_CLIENTS | level_flags::UPDATE_KNOWN_SHAPE;
    if game.level_set_with_flags(foot, foot_state, quiet).is_err() || game.level_set(head, head_state).is_err() {
        return;
    }
    set_supporting(game, rope, true);
    set_supporting(game, far, true);
    sound(game, "minecraft:block.wool.place", foot);
    game.award_stat("used", &item.item, 1);
    game.consume_held();
}

/// Breaking a rope that holds a hammock drops the hammock (RopeAndNailBlock.dropHammock) and
/// frees the rope at its other end; breaking a hammock frees both ropes
/// (HammockBlock.playerWillDestroy). The block itself breaks as usual afterwards.
pub(crate) fn breaking(game: &mut Game<'_>, pos: [i32; 3]) {
    let hammock_at = if &*name_at(game, pos) == ROPE {
        if prop(game, pos, "supporting").as_deref() != Some("true") {
            return;
        }
        let Some(facing) = prop(game, pos, "facing") else { return };
        let front = add(pos, facing_offset(&facing), 1);
        if kind_at(game, front) != Some(Kind::Hammock) {
            return;
        }
        front
    } else if kind_at(game, pos) == Some(Kind::Hammock) {
        pos
    } else {
        return;
    };
    let Some((head, foot, facing)) = halves(game, hammock_at) else { return };
    let d = facing_offset(&facing);
    let ropes = [add(foot, d, -1), add(head, d, 1)];
    if hammock_at != pos {
        let item = name_at(game, head);
        let quiet = level_flags::UPDATE_CLIENTS | level_flags::UPDATE_KNOWN_SHAPE;
        let air = game.world.air;
        let _ = game.level_set_with_flags(head, air, quiet);
        let _ = game.level_set_with_flags(foot, air, quiet);
        game.pop_resource(head, ItemStack::new(&*item, 1));
    }
    for r in ropes {
        if r != pos {
            set_supporting(game, r, false);
        }
    }
}

/// LivingEntity.tick's checkBedExists: our blocks are beds too.
pub(crate) fn still_bed(game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> bool {
    kind_of(game, state).is_some()
}

/// setPosToBed for a guest who fell asleep: `at` is the head's centre at a bed's height
/// (9/16 + 1/8); over a bag or hammock it is the top of that block's own shape + 1/8.
pub(crate) fn sleep_top(game: &Game<'_>, mut at: [f64; 3]) -> [f64; 3] {
    let head = [at[0].floor() as i32, (at[1] - 0.6875).round() as i32, at[2].floor() as i32];
    if kind_at(game, head).is_some()
        && let Some(top) = game.sleep_height(head)
    {
        at[1] = f64::from(head[1]) + top + 0.125;
    }
    at
}

/// The window's own player asleep in a bag or hammock (the client's setPosToBed, which
/// tick_remote_level redoes every tick at a bed's 9/16): the top of the block's own shape.
pub(crate) fn client_tick(game: &mut Game<'_>) {
    let Some(head) = game.sleeping.filter(|h| kind_at(game, *h).is_some()) else {
        return;
    };
    let Some(top) = game.sleep_height(head) else { return };
    if let Some(p) = game.player.as_mut() {
        p.position.y = f64::from(head[1]) + top + 0.125;
    }
}
