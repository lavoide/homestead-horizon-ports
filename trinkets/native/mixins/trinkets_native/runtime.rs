// Trinkets 3.7.2 (Emi, MIT): the player's extra equipment slots, as Homestead's mods declare them
// (tools/trinkets_port.py writes the table, `trinkets_data.rs`). Injected as `crate::game::trinkets`.
//
// - The worn stacks are a player attachment, `trinkets:worn`: a compound of slot id -> item NBT,
//   saved with the player, kept through death when nothing drops it, synced to the player's
//   Horizon client (the cape's elytra glides there). Any crate can read it by declaring the same
//   `AttachmentType<Compound>` (the pair namespace/name is the key): native/things does.
// - The menu (TrinketInventoryScreen's slots) is a server menu, opened by the Trinkets key: a click
//   on an inventory item wears it, a click on a worn one takes it off.
// - A right click with a trinket wears it in the first empty slot that takes it (TrinketItem.use).
// - On death the trinkets drop (DropRule.DEFAULT; vanishing curse destroys them), or go into the
//   grave (native/universalgraves), unless keepInventory.
use super::*;
use horizon_world::attachments::{AttachmentCodec, AttachmentError, AttachmentType, Holder};
use horizon_world::nbt::{Compound, Tag};
use std::collections::HashMap;
use std::sync::Mutex;

pub(crate) use super::trinkets_data::SLOTS;

fn encode(c: &Compound) -> std::result::Result<Compound, AttachmentError> {
    Ok(c.clone())
}
fn decode(c: &Compound) -> std::result::Result<Compound, AttachmentError> {
    Ok(c.clone())
}
fn empty() -> Compound {
    Compound::new()
}

/// The worn trinkets of a player: slot id ("chest/cape") -> the stack's NBT.
pub(crate) static WORN: AttachmentType<Compound> = AttachmentType {
    namespace: "trinkets",
    name: "worn",
    default: empty,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: true,
    sync: true,
};

/// The open trinket menu's window of each player.
static MENUS: Mutex<Option<HashMap<u128, i32>>> = Mutex::new(None);

fn menus<R>(f: impl FnOnce(&mut HashMap<u128, i32>) -> R) -> R {
    let mut g = MENUS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    f(g.get_or_insert_with(HashMap::new))
}

/// Whether slot `i` takes `item` (its trinkets tag, which `all.json` adds to).
pub(crate) fn accepts(i: usize, item: &str) -> bool {
    SLOTS.get(i).is_some_and(|s| s.3.contains(&item))
}

/// Whether some slot takes `item`.
pub(crate) fn is_trinket(item: &str) -> bool {
    SLOTS.iter().any(|s| s.3.contains(&item))
}

/// The player's worn stacks, one per slot of `SLOTS` (none where nothing is worn, or when the
/// player can't be reached).
pub(crate) fn worn(game: &mut Game<'_>, uuid: u128) -> Vec<Option<ItemStack>> {
    let host: &mut dyn horizon_client::mod_api::Host = game;
    let c = WORN.get(host, Holder::Player(uuid)).unwrap_or_default();
    SLOTS
        .iter()
        .map(|s| c.get(&s.0.into()).and_then(horizon_client::save::item_from_nbt))
        .collect()
}

/// Stores the player's worn stacks (one per slot of `SLOTS`).
pub(crate) fn set_worn(game: &mut Game<'_>, uuid: u128, stacks: &[Option<ItemStack>]) -> bool {
    let mut c = Compound::new();
    for (s, stack) in SLOTS.iter().zip(stacks) {
        if let Some(stack) = stack.as_ref().filter(|s| s.count > 0) {
            c.insert(s.0.into(), horizon_client::save::item_to_nbt(stack));
        }
    }
    let host: &mut dyn horizon_client::mod_api::Host = game;
    WORN.set(host, Holder::Player(uuid), c).is_ok()
}

/// The worn stacks of the player in their turn (the acting player), or of the window's own player.
pub(crate) fn worn_here(game: &mut Game<'_>) -> Vec<Option<ItemStack>> {
    match game.lan_acting_uuid.or_else(|| own_uuid(game)) {
        Some(u) => worn(game, u),
        None => vec![None; SLOTS.len()],
    }
}

fn own_uuid(game: &Game<'_>) -> Option<u128> {
    match &game.remote {
        Some(r) => Some(r.profile()),
        None => game.level.player_uuid.map(|i| i.iter().fold(0u128, |a, &v| a << 32 | u128::from(v as u32))),
    }
}

// ---- the menu ----

fn lore_line(text: &str) -> Tag {
    Tag::compound([
        ("text", Tag::String(text.into())),
        ("color", Tag::String("gray".into())),
        ("italic", Tag::Byte(0)),
    ])
}

/// What the menu shows: each worn stack, or the slot's placeholder named for the slot.
fn menu_items(worn: &[Option<ItemStack>]) -> Vec<Option<ItemStack>> {
    let mut out: Vec<Option<ItemStack>> = SLOTS
        .iter()
        .zip(worn)
        .map(|(s, w)| {
            w.clone().or_else(|| {
                let mut p = ItemStack { item: s.2.into(), count: 1, damage: 0, extra: None };
                let d = p.data_mut();
                d.custom_name = Some(Tag::compound([
                    ("text", Tag::String(format!("{} (empty)", s.1).as_str().into())),
                    ("italic", Tag::Byte(0)),
                ]));
                d.other.push((
                    "minecraft:lore".to_owned(),
                    Tag::List(vec![lore_line("Click a trinket in your inventory to wear it.")]),
                ));
                Some(p)
            })
        })
        .collect();
    out.resize(rows() as usize * 9, None);
    out
}

fn rows() -> u8 {
    SLOTS.len().div_ceil(9).clamp(1, 6) as u8
}

/// Opens the trinket menu for the player (the Trinkets key's message).
pub(crate) fn open(game: &mut Game<'_>, uuid: u128) {
    let worn = worn(game, uuid);
    let title = horizon_client::text::Text::literal("Trinkets");
    if let Some(window) = game.native_open_menu(uuid, rows(), title, menu_items(&worn)) {
        menus(|m| m.insert(uuid, window));
    }
}

/// A trinket menu closed.
pub(crate) fn closed(uuid: u128, window: i32) {
    menus(|m| {
        if m.get(&uuid) == Some(&window) {
            m.remove(&uuid);
        }
    });
}

/// A click in the player's trinket menu: true when it was the trinket menu's.
pub(crate) fn clicked(game: &mut Game<'_>, uuid: u128, window: i32, slot: i32) -> bool {
    if menus(|m| m.get(&uuid) != Some(&window)) {
        return false;
    }
    let size = rows() as usize * 9;
    let mut worn = worn(game, uuid);
    let Some(host) = game.lan.as_mut() else { return true };
    let Some(g) = host.hub.guests.iter_mut().find(|g| g.uuid == uuid) else { return true };
    g.inventory.resize(36, None);
    let message;
    if let Ok(i) = usize::try_from(slot)
        && i < SLOTS.len()
    {
        // TrinketSlot: take a worn stack off, into the inventory.
        let Some(stack) = worn[i].take() else { return true };
        match g.inventory.iter().position(Option::is_none) {
            Some(free) => {
                g.inventory[free] = Some(stack);
                message = None;
            }
            None => {
                worn[i] = Some(stack);
                message = Some("No room in your inventory");
            }
        }
    } else if let Some(inv) = super::native_menus::native_menu_inventory_slot(size, slot) {
        // A shift-click from the inventory (TrinketInventoryScreen's quick move): into the first
        // empty slot that takes it, else swapped with the first one that does.
        let Some(stack) = g.inventory[inv].clone() else { return true };
        let fits: Vec<usize> = (0..SLOTS.len()).filter(|&i| accepts(i, &stack.item)).collect();
        match fits.iter().copied().find(|&i| worn[i].is_none()).or(fits.first().copied()) {
            Some(i) if stack.count == 1 => {
                // One item: it and the worn one trade places.
                g.inventory[inv] = worn[i].replace(stack);
                message = None;
            }
            Some(i) => {
                // One of a stack: the worn one (if any) needs a free slot of its own.
                let free = g.inventory.iter().position(Option::is_none);
                if worn[i].is_some() && free.is_none() {
                    message = Some("No room in your inventory");
                } else {
                    let mut one = stack.clone();
                    one.count = 1;
                    if let Some(s) = g.inventory[inv].as_mut() {
                        s.count -= 1;
                    }
                    if let (Some(old), Some(free)) = (worn[i].replace(one), free) {
                        g.inventory[free] = Some(old);
                    }
                    message = None;
                }
            }
            None => message = Some("That doesn't go in a trinket slot"),
        }
    } else {
        return true;
    }
    set_worn(game, uuid, &worn);
    game.native_set_menu_items(uuid, menu_items(&worn));
    if let Some(m) = message {
        game.native_action_bar(uuid, horizon_client::text::Text::literal(m));
    }
    true
}

// ---- wearing from the hand ----

/// TrinketItem.use: a right click with a trinket wears it in the first empty slot that takes it.
pub(crate) fn use_item(game: &mut Game<'_>, stack: &ItemStack) -> bool {
    if !is_trinket(&stack.item) {
        return false;
    }
    let Some(uuid) = game.lan_acting_uuid else { return false };
    let mut worn = worn(game, uuid);
    let Some(i) = (0..SLOTS.len()).find(|&i| worn[i].is_none() && accepts(i, &stack.item)) else {
        return false;
    };
    let Some(held) = game.held_mut().take() else { return false };
    let mut one = held.clone();
    one.count = 1;
    if held.count > 1 {
        let mut rest = held;
        rest.count -= 1;
        *game.held_mut() = Some(rest);
    }
    worn[i] = Some(one);
    set_worn(game, uuid, &worn);
    if let Some(p) = game.player.as_ref() {
        let at = [p.position.x, p.position.y, p.position.z];
        // Player.getSoundSource: PLAYERS, as an armour piece put on (player_equip.rs).
        let players = horizon_client::settings::SOUND_SOURCES.iter().position(|s| *s == "player").unwrap_or(0);
        game.audio.play_on(players, "minecraft:item.armor.equip_generic", Some(at), 1.0, 1.0);
    }
    true
}

// ---- death ----

/// TrinketsApi's death drops for the dying player (their turn): the worn stacks, minus the
/// vanishing-cursed ones, taken out of the slots. Nothing under keepInventory.
pub(crate) fn take_for_death(game: &mut Game<'_>) -> Vec<(usize, ItemStack)> {
    if game.ui.rules.bool("keep_inventory") {
        return Vec::new();
    }
    let Some(uuid) = game.lan_acting_uuid else { return Vec::new() };
    let worn = worn(game, uuid);
    if worn.iter().all(Option::is_none) {
        return Vec::new();
    }
    set_worn(game, uuid, &vec![None; SLOTS.len()]);
    worn.into_iter()
        .enumerate()
        .filter_map(|(i, s)| s.map(|s| (i, s)))
        .filter(|(_, s)| s.enchantment_level("minecraft:vanishing_curse") == 0)
        .collect()
}

/// Player.dropInventory's trinket half when no grave took them: they drop where the player died.
pub(crate) fn drop_on_death(game: &mut Game<'_>, at: [f64; 3]) {
    let pos = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    for (_, s) in take_for_death(game) {
        game.pop_resource(pos, s);
    }
}

/// A grave's slot-exact return (native/universalgraves): back into slot `i` if it is empty and
/// still takes the stack, else none (the grave then gives it like any other item).
pub(crate) fn put_back(game: &mut Game<'_>, i: usize, stack: ItemStack) -> Option<ItemStack> {
    let Some(uuid) = game.lan_acting_uuid else { return Some(stack) };
    let mut worn = worn(game, uuid);
    if i >= SLOTS.len() || worn[i].is_some() || !accepts(i, &stack.item) {
        return Some(stack);
    }
    worn[i] = Some(stack);
    set_worn(game, uuid, &worn);
    None
}

// ---- the key ----

/// The Trinkets key (`key.trinkets.open`, G unless the player chose another), while playing or
/// in the inventory: asks the server for the menu.
pub(crate) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed {
        return;
    }
    if key != game.native_mod_key("trinkets", "open", "key.keyboard.g") {
        return;
    }
    if !(game.native_playing() || game.screen == Screen::Inventory) {
        return;
    }
    let _ = game.native_net_send_to_server("trinkets:open", &[]);
}

/// The server hears the key's message.
pub(crate) fn net(game: &mut Game<'_>, player: u128, channel: &str) {
    if channel == "trinkets:open" {
        open(game, player);
    }
}

// ---- a worn Totem of Undying ----
// Friends & Foes declares the charm slot for totems (its TotemHelper checks it after the hands).
// After native/inventorytotem's own search (the same targets, which it wins on mod id), a worn
// totem saves the player too.

const TOTEM: &str = "minecraft:totem_of_undying";

/// LivingEntity.checkTotemDeathProtection's readiness: a worn totem counts.
pub(crate) fn totem_ready(game: &mut Game<'_>) -> bool {
    if !worn_here(game).iter().flatten().any(|s| &*s.item == TOTEM) {
        return false;
    }
    if let Some(p) = game.player.as_mut() {
        p.health.totem_ready = true;
    }
    true
}

/// The death protection from a worn totem: it moves to the off hand for the original to use up.
pub(crate) fn apply_worn_totem(game: &mut Game<'_>) -> Option<Option<super::totem::Body>> {
    let used = game.player.as_ref().is_some_and(|p| p.health.totem_used);
    let in_hand = game.held().is_some_and(|s| &*s.item == TOTEM) || game.offhand.as_ref().is_some_and(|s| &*s.item == TOTEM);
    if !used || in_hand {
        return None;
    }
    let uuid = game.lan_acting_uuid.or_else(|| own_uuid(game))?;
    let mut worn = worn(game, uuid);
    let i = worn.iter().position(|s| s.as_ref().is_some_and(|s| &*s.item == TOTEM))?;
    let totem = worn[i].take();
    set_worn(game, uuid, &worn);
    let saved = std::mem::replace(&mut game.offhand, totem);
    let body = game.apply_totem();
    game.offhand = saved;
    Some(body)
}

// ---- the tooltip ----

/// TrinketsClient's tooltip: which trinket slots take the item (placeholders show nothing).
pub(crate) fn tooltip(stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    let names: Vec<&str> = SLOTS.iter().filter(|s| s.3.contains(&&*stack.item)).map(|s| s.1).collect();
    if names.is_empty() || stack.item.starts_with("trinkets:slot_") {
        return;
    }
    let grey = 0xFFAAAAAA;
    let line = if names.len() == 1 {
        format!("Equippable in the §9{}§7 trinket slot (G)", names[0])
    } else {
        let mut n = names.clone();
        n.dedup();
        format!("Equippable in the trinket slots: §9{}§7 (G)", n.join(", "))
    };
    lines.push((horizon_client::text::Text::legacy(&format!("§7{line}")), grey));
}
