//! Things 0.3.3 (glisco, MIT), the trait half: what the trinkets and tools do. The items are the
//! content mod mods/things (tools/things_port.py); the sock recipes, the Bater Wucket and the
//! Hardening Catalyst are the bundle's mixins.
//!
//! Horizon has no Trinkets slots, so a trinket counts as worn from the hotbar or the off hand,
//! one per Trinkets slot Things uses (face, necklace, glove, belt, shoes), the off hand first,
//! then the hotbar from the left. Homestead's things.json5: mossNecklaceRegen 2,
//! miningGloveMomentum 2, riotGauntletStrength 1, sockPerLevelSpeedAmplifier 0.02,
//! infernalScepterDurability 64 (the content mod's max damage).
//!
//! Effects are commands, run at the next server tick (a player's hook can't run commands); the
//! attribute modifiers are the bundle's game mixin (`/attribute` can't reach a player from
//! there). The effects are refreshed every 10 ticks with 2 s left, where Java gives
//! 5 ticks every tick, so one lingers up to 2 s after the trinket leaves the hotbar.
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Flow, Holder, Hooks, Host,
    InventoryTickEvent, KeyEvent, LivingEntityUseItemFinishEvent, LivingIncomingDamageEvent,
    NetFromClientEvent, PlayerTickEvent, ServerTickEvent, Tag, UseItemEvent, UseItemOnEvent,
};
use horizon_client::text::Text;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod { state: Mutex::new(State::default()) })
}

const OFF_HAND: i32 = -106;
const REFRESH: i64 = 10;
const MOSS_REGEN: i32 = 2;
const MINING_MOMENTUM: i32 = 2;
const RIOT_STRENGTH: i32 = 1;
const MAGNET_COST: u32 = 50;
const MAGNET_CHARGE: u32 = 200;
const SCEPTER_USES: u32 = 64;

/// What a player wears, as of their last tick.
#[derive(Clone, Default, PartialEq)]
struct Worn {
    items: HashSet<String>,
    /// Socks: speed level and whether they're jumpy and the jump boost is on.
    socks: Option<(i32, bool, bool)>,
}

impl Worn {
    fn has(&self, item: &str) -> bool {
        self.items.contains(item)
    }
}

#[derive(Default)]
struct Player {
    worn: Worn,
    ticks: i64,
    /// When the Mossy Necklace last gave Regeneration (its 610 ticks).
    regen_at: Option<i64>,
}

#[derive(Default)]
struct State {
    queue: Vec<String>,
    players: HashMap<u128, Player>,
    /// Network ids of players wearing the Shock Absorber, for the damage hook.
    shock: HashSet<i32>,
    rng: u64,
    /// Container key uses waiting for the tick: player, name, key, position.
    keys: Vec<(u128, String, i32, [i32; 3])>,
    /// Locked containers: position and dimension to key (also saved in the world folder).
    locks: Option<HashMap<String, i32>>,
}

impl State {
    fn float(&mut self) -> f64 {
        if self.rng == 0 {
            self.rng = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) | 1;
        }
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }
}

struct Mod {
    state: Mutex<State>,
}

impl Mod {
    fn st(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn custom(stack: &horizon_client::mod_api::ItemStack) -> Option<&Tag> {
    stack.data()?.other.iter().find(|(k, _)| k == "minecraft:custom_data").map(|(_, t)| t)
}

fn int_of(tag: Option<&Tag>, key: &str) -> Option<i32> {
    match tag?.get(key)? {
        Tag::Int(v) => Some(*v),
        Tag::Byte(v) => Some(i32::from(*v)),
        Tag::Short(v) => Some(i32::from(*v)),
        _ => None,
    }
}

fn set_custom(stack: &mut horizon_client::mod_api::ItemStack, key: &str, value: Tag) {
    let d = stack.data_mut();
    match d.other.iter_mut().find(|(k, _)| k == "minecraft:custom_data") {
        Some((_, Tag::Compound(c))) => {
            c.insert(key.into(), value);
        }
        _ => {
            d.other.retain(|(k, _)| k != "minecraft:custom_data");
            let mut c = Compound::new();
            c.insert(key.into(), value);
            d.other.push(("minecraft:custom_data".to_owned(), Tag::Compound(c)));
        }
    }
}

/// A lore line as the content mod writes them: gray, not italic.
fn lore_line(text: &str) -> Tag {
    Tag::compound([
        ("text", Tag::String(text.into())),
        ("color", Tag::String("gray".into())),
        ("italic", Tag::Byte(0)),
    ])
}

fn set_lore(stack: &mut horizon_client::mod_api::ItemStack, lines: &[String]) {
    let d = stack.data_mut();
    d.other.retain(|(k, _)| k != "minecraft:lore");
    d.other.push(("minecraft:lore".to_owned(), Tag::List(lines.iter().map(|l| lore_line(l)).collect())));
}

/// The socks' tooltip (SocksItem.appendTooltip) as lore.
pub fn socks_lore(speed: i32, jumpy: bool, disabled: bool) -> Vec<String> {
    let mut lines = Vec::new();
    if jumpy {
        lines.push(format!("↑ Jumpy{}", if disabled { " (off: Caps Lock to toggle)" } else { " (Caps Lock to toggle)" }));
    }
    lines.push(match speed {
        0 => "☄ Average".to_owned(),
        1 => "☄ Speedy".to_owned(),
        2 => "☄ Usain Bolt".to_owned(),
        n => format!("☄ Naughty ({n})"),
    });
    lines.push("Wear it in a trinket slot (G).".to_owned());
    lines
}

/// The hand slots the Item Magnet and the Infernal Scepter are found in: the off hand, then the hotbar.
const WORN_SLOTS: [i32; 10] = [OFF_HAND, 0, 1, 2, 3, 4, 5, 6, 7, 8];

fn same(c: &Compound) -> Result<Compound, AttachmentError> {
    Ok(c.clone())
}
fn no_trinkets() -> Compound {
    Compound::new()
}
/// native/trinkets' worn trinkets of a player (slot id -> item NBT): declared with the same key and
/// type, so it reads the same value.
static TRINKETS: AttachmentType<Compound> = AttachmentType {
    namespace: "trinkets",
    name: "worn",
    default: no_trinkets,
    codec: Some(AttachmentCodec { encode: same, decode: same }),
    copy_on_death: true,
    sync: true,
};

/// The stacks in the player's trinket slots.
fn trinket_stacks(host: &mut dyn Host, uuid: u128) -> Vec<horizon_client::mod_api::ItemStack> {
    let c = TRINKETS.get(host, Holder::Player(uuid)).unwrap_or_default();
    c.values().filter_map(horizon_client::save::item_from_nbt).collect()
}

/// What the player wears in their trinket slots (TrinketComponent.isEquipped).
fn worn(host: &mut dyn Host, uuid: u128) -> Worn {
    let mut out = Worn::default();
    for stack in trinket_stacks(host, uuid) {
        if &*stack.item == "things:socks" {
            let c = custom(&stack);
            out.socks = Some((
                int_of(c, "Speed").unwrap_or(0),
                int_of(c, "Jumpy").unwrap_or(0) != 0,
                int_of(c, "JumpBoostDisabled").unwrap_or(0) != 0,
            ));
        }
        out.items.insert(stack.item.to_string());
    }
    out
}

fn effect(name: &str, effect: &str, seconds: i32, amplifier: i32) -> String {
    format!("effect give {name} {effect} {seconds} {amplifier} true")
}

fn int_array(uuid: u128) -> Tag {
    Tag::IntArray(vec![(uuid >> 96) as i32, (uuid >> 64) as i32, (uuid >> 32) as i32, uuid as i32])
}

fn look(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

fn passable(block: &str) -> bool {
    matches!(block, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" | "minecraft:water" | "minecraft:lava" | "minecraft:bubble_column")
}

/// A lockable container (LockableContainerBlockEntity's blocks).
fn lockable(block: &str) -> bool {
    let b = block.trim_start_matches("minecraft:");
    !b.contains("ender_chest")
        && (b.ends_with("chest")
            || b == "barrel"
            || b.ends_with("furnace")
            || b == "smoker"
            || b == "hopper"
            || b == "dispenser"
            || b == "dropper"
            || b.ends_with("shulker_box")
            || b == "brewing_stand"
            || b == "crafter")
}

fn pretty(block: &str) -> String {
    block
        .rsplit(':')
        .next()
        .unwrap_or(block)
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn lock_key(dimension: &str, p: [i32; 3]) -> String {
    format!("{dimension} {} {} {}", p[0], p[1], p[2])
}

fn locks_file(host: &dyn Host) -> Option<std::path::PathBuf> {
    host.world_dir().map(|d| d.join("things").join("locks.json"))
}

fn load_locks(host: &dyn Host) -> HashMap<String, i32> {
    locks_file(host)
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| horizon_client::mod_api::serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_locks(host: &dyn Host, locks: &HashMap<String, i32>) {
    if let Some(f) = locks_file(host) {
        let _ = std::fs::create_dir_all(f.parent().unwrap_or(&f));
        if let Ok(b) = horizon_client::mod_api::serde_json::to_vec_pretty(locks) {
            let _ = std::fs::write(f, b);
        }
    }
}

/// ContainerKeyItem.maybeGetOtherChest: a double chest's other half.
fn other_half(host: &dyn Host, p: [i32; 3]) -> Option<[i32; 3]> {
    let kind = host.block_property(p, "type")?;
    let facing = host.block_property(p, "facing")?;
    // ChestBlock.getFacing: LEFT → facing clockwise, RIGHT → counterclockwise.
    let (cw, ccw) = match facing.as_str() {
        "north" => ([1, 0], [-1, 0]),
        "east" => ([0, 1], [0, -1]),
        "south" => ([-1, 0], [1, 0]),
        "west" => ([0, -1], [0, 1]),
        _ => return None,
    };
    let d = match kind.as_str() {
        "left" => cw,
        "right" => ccw,
        _ => return None,
    };
    Some([p[0] + d[0], p[1], p[2] + d[1]])
}

impl Mod {
    fn tick_player(&self, host: &mut dyn Host) {
        let Some(p) = host.acting_player() else { return };
        let now = worn(host, p.uuid);
        let light = {
            let at = [p.position[0].floor() as i32, p.position[1].floor() as i32, p.position[2].floor() as i32];
            host.light(at)
        };
        let day = host.day_time().rem_euclid(24000);
        let mut st = self.st();
        let mut queue = Vec::new();
        let name = p.name.clone();
        {
            let pl = st.players.entry(p.uuid).or_default();
            pl.ticks += 1;
            let before = std::mem::replace(&mut pl.worn, now.clone());
            // Unequipping (onUnequip): the effects Java takes away.
            if before.has("things:monocle") && !now.has("things:monocle") {
                queue.push(format!("effect clear {name} minecraft:night_vision"));
            }
            if before.has("things:moss_necklace") && !now.has("things:moss_necklace") {
                queue.push(format!("effect clear {name} minecraft:regeneration"));
                pl.regen_at = None;
            }
            if pl.ticks % REFRESH == 0 && p.game_mode != 3 {
                if now.has("things:monocle") {
                    queue.push(effect(&name, "minecraft:night_vision", 31, 0));
                }
                if now.has("things:mining_gloves") {
                    // The Momentum effect is Haste that stacks with a beacon's; Haste here.
                    queue.push(effect(&name, "minecraft:haste", 2, MINING_MOMENTUM - 1));
                }
                if now.has("things:riot_gauntlet") {
                    queue.push(effect(&name, "minecraft:strength", 2, RIOT_STRENGTH - 1));
                }
                if now.has("things:rabbit_foot_charm") || now.socks.is_some_and(|s| s.1 && !s.2) {
                    queue.push(effect(&name, "minecraft:jump_boost", 2, 1));
                }
                if now.has("things:hades_crystal") {
                    queue.push(effect(&name, "minecraft:fire_resistance", 2, 0));
                }
                if now.has("things:moss_necklace") {
                    // MossNecklaceItem.tick: block light over 7, or sky light over 7 by day;
                    // Regeneration for 610 ticks once the last one is nearly over.
                    let lit = light.0 > 7 || (light.1 > 7 && (day > 23500 || day < 12500));
                    let due = pl.regen_at.is_none_or(|t| pl.ticks - t >= 600);
                    if lit && due {
                        queue.push(effect(&name, "minecraft:regeneration", 30, MOSS_REGEN - 1));
                        pl.regen_at = Some(pl.ticks);
                    }
                }
            }
        }
        if now.has("things:shock_absorber") {
            st.shock.insert(p.entity_id);
        } else {
            st.shock.remove(&p.entity_id);
        }
        st.queue.extend(queue);
    }

    fn use_magnet(&self, host: &mut dyn Host, damage: u32) -> Flow {
        let charge = MAGNET_CHARGE.saturating_sub(damage);
        if charge < MAGNET_COST {
            return Flow::Cancel;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        let dir = look(p.yaw, p.pitch);
        let eye = [p.position[0], p.position[1] + 1.62, p.position[2]];
        // ItemMagnetItem.use: along the view from 2 to 10 blocks, until a block, the items
        // within 1.25 + charge / 200 * 2 of each point come to the player.
        let radius = 1.25 + f64::from(charge) / 200.0 * 2.0;
        let mut queue = Vec::new();
        let mut d = 2.0;
        while d < 10.0 {
            let at = [eye[0] + dir[0] * d, eye[1] + dir[1] * d, eye[2] + dir[2] * d];
            let block = host.block([at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32]);
            if !passable(&block) {
                break;
            }
            queue.push(format!(
                "execute in {} positioned {:.2} {:.2} {:.2} as @e[type=item,distance=..{radius:.2}] run tp @s {}",
                p.dimension, at[0], at[1], at[2], p.name
            ));
            d += 1.0;
        }
        queue.push(format!(
            "execute as {} at @s as @e[type=item,distance=..1.5] run data merge entity @s {{PickupDelay:0s}}",
            p.name
        ));
        queue.push(format!("execute as {0} at @s run playsound minecraft:entity.enderman.teleport player {0} ~ ~ ~ 0.125 2", p.name));
        for slot in WORN_SLOTS {
            if let Some(s) = host.carried_stack(slot)
                && &*s.item == "things:item_magnet"
                && s.damage == damage
            {
                s.damage = (damage + MAGNET_COST).min(MAGNET_CHARGE);
                break;
            }
        }
        self.st().queue.extend(queue);
        Flow::Cancel
    }

    fn use_scepter(&self, host: &mut dyn Host, damage: u32) -> Flow {
        let Some(p) = host.acting_player() else { return Flow::Continue };
        if host.count_items("minecraft:fire_charge") == 0 && p.game_mode != 1 {
            return Flow::Continue;
        }
        // InfernalScepterItem.onStoppedUsing: a fireball of power 3 two blocks ahead at eye
        // height minus 1, flying along the view.
        let dir = look(p.yaw, p.pitch);
        let at = [p.position[0] + dir[0] * 2.0, p.position[1] + 1.62 - 1.0, p.position[2] + dir[2] * 2.0];
        let owner = int_array(p.uuid);
        let Tag::IntArray(o) = owner else { return Flow::Continue };
        let motion: Vec<String> = dir.iter().map(|v| format!("{:.5}d", v * 0.1)).collect();
        self.st().queue.push(format!(
            "execute in {} run summon minecraft:fireball {:.3} {:.3} {:.3} {{ExplosionPower:3b,acceleration_power:0.1d,Motion:[{}],Owner:[I;{},{},{},{}]}}",
            p.dimension, at[0], at[1], at[2], motion.join(","), o[0], o[1], o[2], o[3]
        ));
        if p.game_mode != 1 {
            host.take_items("minecraft:fire_charge", 1);
            for slot in WORN_SLOTS {
                if let Some(s) = host.carried_stack(slot)
                    && &*s.item == "things:infernal_scepter"
                    && s.damage == damage
                {
                    s.damage += 1;
                    if s.damage >= SCEPTER_USES {
                        s.count = 0;
                    }
                    break;
                }
            }
        }
        self.st().queue.push(format!(
            "execute in {} run playsound minecraft:entity.ghast.shoot player @a {:.2} {:.2} {:.2} 1 1",
            p.dimension, p.position[0], p.position[1], p.position[2]
        ));
        Flow::Cancel
    }

    /// ThingsClient.TOGGLE_SOCKS_JUMP_BOOST's packet: the worn jumpy socks' jump boost on or off.
    fn toggle_socks(&self, host: &mut dyn Host, uuid: u128) {
        let mut c = TRINKETS.get(host, Holder::Player(uuid)).unwrap_or_default();
        let Some((key, mut s)) = c
            .iter()
            .find_map(|(k, t)| horizon_client::save::item_from_nbt(t).filter(|s| &*s.item == "things:socks").map(|s| (k.clone(), s)))
        else {
            return;
        };
        let cd = custom(&s);
        let (speed, jumpy) = (int_of(cd, "Speed").unwrap_or(0), int_of(cd, "Jumpy").unwrap_or(0) != 0);
        if !jumpy {
            return;
        }
        let off = int_of(cd, "JumpBoostDisabled").unwrap_or(0) == 0;
        set_custom(&mut s, "JumpBoostDisabled", Tag::Byte(i8::from(off)));
        set_lore(&mut s, &socks_lore(speed, true, off));
        c.insert(key, horizon_client::save::item_to_nbt(&s));
        let _ = TRINKETS.set(host, Holder::Player(uuid), c);
        host.action_bar(uuid, Text::literal(if off { "Jump Boost off" } else { "Jump Boost on" }));
    }
}

impl Hooks for Mod {
    /// The socks' key (Caps Lock unless the player chose another), in the window.
    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        if event.pressed && event.key == host.mod_key("things", "toggle_socks_jump_boost", "key.keyboard.caps.lock") {
            let _ = host.net_send_to_server("things:toggle_socks", &[]);
        }
    }

    fn net_from_client(&self, host: &mut dyn Host, event: &mut NetFromClientEvent<'_>) {
        if event.channel == "things:toggle_socks" {
            self.toggle_socks(host, event.player);
        }
    }

    fn player_tick(&self, host: &mut dyn Host, _event: &mut PlayerTickEvent) {
        self.tick_player(host);
    }

    fn inventory_tick(&self, host: &mut dyn Host, event: &mut InventoryTickEvent) {
        let Some(stack) = host.carried_stack(event.slot) else { return };
        match &*stack.item {
            // ItemMagnetItem.inventoryTick: charge + 1 + charge / 80, up to 200.
            "things:item_magnet" if stack.damage > 0 => {
                let charge = MAGNET_CHARGE - stack.damage.min(MAGNET_CHARGE);
                let next = (charge + 1 + charge / 80).min(MAGNET_CHARGE);
                stack.damage = MAGNET_CHARGE - next;
            }
            // ContainerKeyItem.inventoryTick: a new key gets its number.
            "things:container_key" if int_of(custom(stack), "Lock").is_none() => {
                let key = (self.st().float() * 200000.0) as i32;
                set_custom(stack, "Lock", Tag::Int(key));
                set_lore(stack, &[
                    format!("Key: #{key:x}"),
                    "Sneak-use on a container to lock or unlock it.".to_owned(),
                    "Hold the key to open a locked container.".to_owned(),
                ]);
            }
            _ => {}
        }
    }

    fn server_tick(&self, host: &mut dyn Host, _event: &mut ServerTickEvent<'_>) {
        let (pending, keys) = {
            let mut st = self.st();
            (std::mem::take(&mut st.queue), std::mem::take(&mut st.keys))
        };
        for line in pending {
            if !host.run_command(&line) {
                eprintln!("[things] command failed: {line}");
            }
        }
        if !keys.is_empty() {
            self.lock_containers(host, keys);
        }
    }

    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        match &*event.stack.item {
            "things:item_magnet" => self.use_magnet(host, event.stack.damage),
            "things:infernal_scepter" => self.use_scepter(host, event.stack.damage),
            _ => Flow::Continue,
        }
    }

    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if &*event.stack.item != "things:container_key" {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        // ContainerKeyItem.useOnBlock only acts sneaking; this hook only hears a click a
        // container let through, which is a sneaking one.
        if !lockable(&host.block(event.pos)) {
            return Flow::Continue;
        }
        let Some(key) = int_of(custom(event.stack), "Lock") else { return Flow::Continue };
        self.st().keys.push((p.uuid, p.dimension.clone(), key, event.pos));
        Flow::Cancel
    }

    fn living_entity_use_item_finish(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemFinishEvent<'_>) {
        let Some(p) = host.acting_player() else { return };
        if event.entity.is_some_and(|e| e != p.entity_id) {
            return;
        }
        let item = event.stack.item.to_string();
        let mut st = self.st();
        let worn = st.players.get(&p.uuid).map(|pl| pl.worn.clone()).unwrap_or_default();
        match item.as_str() {
            // PotionItemMixin: with the Placebo, a potion 75% of the time isn't used up.
            "minecraft:potion" if worn.has("things:placebo") && p.game_mode != 1 => {
                if st.float() <= 0.75 {
                    *event.result = Some(event.stack.clone());
                }
            }
            // PlayerEntityMixin: a poisonous potato with the Luck of the Irish.
            "minecraft:poisonous_potato" if worn.has("things:luck_of_the_irish") => {
                // The 2 ticks of Saturation are left out: /effect counts whole seconds.
                let n = &p.name;
                st.queue.push(format!("effect give {n} minecraft:regeneration 5 1"));
                st.queue.push(format!("effect give {n} minecraft:luck 20 0"));
                st.queue.push(format!("effect clear {n} minecraft:poison"));
            }
            _ => {}
        }
    }

    fn living_incoming_damage(&self, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if !self.st().shock.contains(&event.entity) {
            return Flow::Continue;
        }
        match event.damage_type {
            // LivingEntityMixin.decreaseFallDamage: minus 20%, at most 16.
            "minecraft:fall" => event.amount -= (event.amount * 0.2).min(16.0).floor(),
            // decreaseKineticDamage: flying into a wall, a quarter.
            "minecraft:fly_into_wall" => event.amount /= 4.0,
            _ => {}
        }
        Flow::Continue
    }
}

impl Mod {
    /// ContainerKeyItem.useOnBlock, at the tick: lock an unlocked container (and its other half)
    /// with the key, unlock one locked with it, or say it has another key. The lock is any
    /// Container Key (Horizon checks that, for guests) with this number in its custom data
    /// (which Horizon's predicate leaves out: the bundle's open_menu hook checks it).
    fn lock_containers(&self, host: &mut dyn Host, keys: Vec<(u128, String, i32, [i32; 3])>) {
        let mut locks = self.st().locks.take().unwrap_or_else(|| load_locks(host));
        for (uuid, dim, key, pos) in keys {
            let mut spots = vec![pos];
            if let Some(o) = other_half(host, pos) {
                spots.push(o);
            }
            // A lock the world lost (the block broken, a /data change) is forgotten.
            for s in &spots {
                let cmd = format!("execute in {dim} if data block {} {} {} lock", s[0], s[1], s[2]);
                if !host.run_command(&cmd) {
                    locks.remove(&lock_key(&dim, *s));
                }
            }
            let existing = spots.iter().find_map(|s| locks.get(&lock_key(&dim, *s)).copied());
            let name = pretty(&host.block(pos));
            let line = match existing {
                None => {
                    for s in &spots {
                        host.run_command(&format!(
                            "execute in {dim} run data merge block {} {} {} {{lock:{{items:\"things:container_key\",predicates:{{\"minecraft:custom_data\":{{Lock:{key}}}}}}}}}",
                            s[0], s[1], s[2]
                        ));
                        locks.insert(lock_key(&dim, *s), key);
                    }
                    format!("{name} locked!")
                }
                Some(k) if k == key => {
                    for s in &spots {
                        host.run_command(&format!("execute in {dim} run data remove block {} {} {} lock", s[0], s[1], s[2]));
                        locks.remove(&lock_key(&dim, *s));
                    }
                    format!("{name} unlocked!")
                }
                Some(_) => format!("{name} is locked with another key!"),
            };
            host.run_command(&format!(
                "execute in {dim} run playsound minecraft:block.chest.locked block @a {} {} {} 1 1",
                pos[0], pos[1], pos[2]
            ));
            host.action_bar(uuid, Text::literal(line));
        }
        save_locks(host, &locks);
        self.st().locks = Some(locks);
    }
}
