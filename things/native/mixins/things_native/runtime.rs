// Things' Bater Wucket (BucketItemMixin.preserverBaterWucket and its cauldron behaviour).
// Injected as `crate::game::things`; the rest of Things is native/things/api.
use super::*;
use core::ops::ControlFlow;
use std::cell::Cell;

thread_local! {
    static WUCKET: Cell<bool> = const { Cell::new(false) };
}

/// The Bater Wucket in the main hand: the water bucket's use (placing water, filling a cauldron),
/// then the wucket back, still full.
pub(crate) fn use_wucket(game: &mut Game<'_>, stack: &ItemStack, allow: bool) -> ControlFlow<Result<horizon_client::first_person::UseResult>> {
    if WUCKET.get() || &*stack.item != "things:bater_wucket" {
        return ControlFlow::Continue(());
    }
    let Some(original) = game.held().cloned().filter(|h| h.item == stack.item) else {
        return ControlFlow::Continue(());
    };
    let mut twin = original.clone();
    twin.item = std::sync::Arc::from("minecraft:water_bucket");
    *game.held_mut() = Some(twin.clone());
    WUCKET.set(true);
    let result = game.use_tool(&twin, allow);
    WUCKET.set(false);
    *game.held_mut() = Some(original);
    ControlFlow::Break(result)
}

// ---- the trinkets' attribute modifiers ----
// The worn trinkets are native/trinkets' slots. `/attribute` only reaches the player whose state is in place, which a trait mod's server tick
// has none of, so the modifiers are set here, transient (gone on leaving, as Trinkets takes them
// off), each player's in their own tick. The rest of the trinkets is native/things/api.

fn same(c: &horizon_world::nbt::Compound) -> std::result::Result<horizon_world::nbt::Compound, horizon_world::attachments::AttachmentError> {
    Ok(c.clone())
}
fn no_trinkets() -> horizon_world::nbt::Compound {
    horizon_world::nbt::Compound::new()
}
/// native/trinkets' worn trinkets (slot id -> item NBT); the same key reads the same value, so
/// this works with or without that bundle built in (nothing worn without it).
static TRINKETS: horizon_world::attachments::AttachmentType<horizon_world::nbt::Compound> =
    horizon_world::attachments::AttachmentType {
        namespace: "trinkets",
        name: "worn",
        default: no_trinkets,
        codec: Some(horizon_world::attachments::AttachmentCodec { encode: same, decode: same }),
        copy_on_death: true,
        sync: true,
    };

/// The acting player's worn trinkets.
fn worn_trinkets(game: &mut Game<'_>) -> Vec<ItemStack> {
    let Some(uuid) = game.lan_acting_uuid else { return Vec::new() };
    let host: &mut dyn horizon_client::mod_api::Host = game;
    let c = TRINKETS.get(host, horizon_world::attachments::Holder::Player(uuid)).unwrap_or_default();
    c.values().filter_map(horizon_client::save::item_from_nbt).collect()
}

fn custom_int(stack: &ItemStack, key: &str) -> i32 {
    let Some(d) = stack.data() else { return 0 };
    match d.other.iter().find(|(k, _)| k == "minecraft:custom_data").and_then(|(_, t)| t.get(key)) {
        Some(horizon_world::nbt::Tag::Int(v)) => *v,
        Some(horizon_world::nbt::Tag::Byte(v)) => i32::from(*v),
        _ => 0,
    }
}

/// SockDataComponent: sockPerLevelSpeedAmplifier (Homestead: 0.02) per level, level + 1.
const SOCK_SPEED: f64 = 0.019999999552965164;

thread_local! {
    /// The modifiers set for each player (LAN uuid, 0 for the host) and when.
    static APPLIED: std::cell::RefCell<std::collections::HashMap<u128, ([f64; 4], i64)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Socks: movement speed; jumpy socks: step height + 0.45 (EntityMixin.boostStepHeight); Arm
/// Extender: reach + 2 (both ranges); Enchanted Wax Gland: full water movement efficiency, in
/// place of LivingEntityMixin's swim speed times 10.
pub(crate) fn trinket_modifiers(game: &mut Game<'_>) {
    let mut want = [0.0f64; 4];
    // The trinket slots (native/trinkets' `trinkets:worn` attachment).
    let stacks = worn_trinkets(game);
    for s in &stacks {
        match &*s.item {
            "things:socks" => {
                want[0] = SOCK_SPEED * f64::from(custom_int(s, "Speed") + 1);
                if custom_int(s, "Jumpy") != 0 {
                    want[1] = 0.45;
                }
            }
            "things:arm_extender" => want[2] = 2.0,
            "things:enchanted_wax_gland" => want[3] = 1.0,
            _ => {}
        }
    }
    let who = game.lan_acting_uuid.unwrap_or(0);
    let now = game.world.game_time as i64;
    let due = APPLIED.with(|a| {
        a.borrow().get(&who).is_none_or(|(had, at)| *had != want || now - at >= 100 || now < *at)
    });
    if !due {
        return;
    }
    let Some(player) = game.player.as_mut() else { return };
    let set = |p: &mut _, attribute: &str, id: &str, v: f64| {
        let _ = horizon_client::motion::PlayerMotion::set_modifier(p, attribute, id, v, "add_value", v != 0.0);
    };
    set(player, "minecraft:movement_speed", "things:socks", want[0]);
    set(player, "minecraft:step_height", "things:jumpy_socks", want[1]);
    set(player, "minecraft:block_interaction_range", "things:arm_extender", want[2]);
    set(player, "minecraft:entity_interaction_range", "things:arm_extender", want[2]);
    set(player, "minecraft:water_movement_efficiency", "things:wax_gland", want[3]);
    APPLIED.with(|a| a.borrow_mut().insert(who, (want, now)));
}

// ---- the Recall Potion ----

/// RecallPotionItem.finishUsing: to the player's own respawn point (a bed or respawn anchor
/// still there, or a forced point), in the level they're in; the world spawn isn't one.
/// Java also takes them to another dimension's point: here they're told it's elsewhere.
pub(crate) fn recall(game: &mut Game<'_>, stack: &ItemStack) {
    if &*stack.item != "things:recall_potion" {
        return;
    }
    let floor = |s: Vec3| [s.x.floor() as i32, s.y.floor() as i32, s.z.floor() as i32];
    let target = match &game.respawn_config {
        Some(c) => {
            let name = game.res.facts().get(game.world.block(c.pos[0], c.pos[1], c.pos[2])).name.clone();
            let bed = name.ends_with("_bed");
            let anchor = &*name == "minecraft:respawn_anchor";
            if !(c.forced || bed || anchor) {
                Err("No respawn point")
            } else if c.dim != game.dim {
                Err("Your respawn point is in another dimension")
            } else {
                let up = if bed { 0.5625 } else if anchor { 1.0 } else { 0.0 };
                Ok(Vec3::new(f64::from(c.pos[0]) + 0.5, f64::from(c.pos[1]) + up, f64::from(c.pos[2]) + 0.5))
            }
        }
        None => match game.spawn {
            Some(s) if Some(floor(s)) != game.level.world_spawn => {
                if game.dim == Dim::Overworld { Ok(s) } else { Err("Your respawn point is in another dimension") }
            }
            _ => Err("No respawn point"),
        },
    };
    match target {
        Ok(v) => {
            // As /tp places a player in their level (commands.rs teleport_to_in_level).
            if let Some(p) = game.player.as_mut() {
                p.position = v;
                p.velocity = Vec3::new(0.0, 0.0, 0.0);
                p.fall_distance = 0.0;
                p.ground.on_ground = false;
            }
            game.previous_position = v;
        }
        Err(line) => game.system_message(horizon_client::text::Text::literal(line)),
    }
}

// ---- locked containers for the host ----

/// `BaseContainerBlockEntity.canOpen` where Horizon doesn't check it: a container's `lock`
/// keeps only a remote guest out (lan_actions), not the host's own player (whose open comes
/// here, as connection 1), so the Container Key's locks wouldn't hold for them. Both halves of a
/// double chest count, as `ChestBlock`'s menu provider asks. A guest checked there is checked
/// again here.
pub(crate) fn host_locked(game: &mut Game<'_>, pos: [i32; 3]) -> ControlFlow<()> {
    let key = (pos[0], pos[1], pos[2]);
    let hand = game.held().cloned();
    let spectator = game.spectator();
    let mut keys = vec![key];
    if let Some((other, _)) = game.chest_partner(key) {
        keys.push(other);
    }
    // Horizon's ItemPredicate leaves out `minecraft:custom_data`, which a Container Key's lock
    // names (`predicates`: its number): that part is checked here.
    let held_key = hand.as_ref().map(|h| custom_int(h, "Lock"));
    let wrong_key = |k: &(i32, i32, i32)| {
        let Some(lock) = game.world.block_lock(*k) else { return false };
        let want = match lock.get("predicates").and_then(|p| p.get("minecraft:custom_data")).and_then(|c| c.get("Lock")) {
            Some(horizon_world::nbt::Tag::Int(v)) => *v,
            _ => return false,
        };
        !spectator && held_key != Some(want)
    };
    if !keys.iter().any(|k| game.container_locked_for(*k, spectator, hand.as_ref()) || wrong_key(k)) {
        return ControlFlow::Continue(());
    }
    let name = game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name.clone();
    let title = horizon_client::text::Text::translate(&format!("block.{}", name.replace(':', ".")), &[]);
    game.system_message(horizon_client::text::Text::translate("container.isLocked", &[title]));
    ControlFlow::Break(())
}
