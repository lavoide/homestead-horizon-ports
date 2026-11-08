// Mouse Wheelie 1.16.3 (Siphalor, Apache-2.0): ContainerScreenHelper, InventorySorter and ToolPicker,
// with Homestead's config (mousewheelie.hjson): directional scrolling on, soft hotbar scoping, the drop
// modifier on, sort modes creative / Shift quantity / Ctrl alphabet, item kinds matched ignoring damage
// and enchantments (SOME), tool picking holds tools and picks from the whole inventory.
//
// Mouse Wheelie's default keys: whole stack Shift, all of a kind Ctrl, drop Alt, deposit and restock
// Space, sort the middle button. Every action is plain slot clicks through `tweak_click`, which
// predicts and sends them on a server and runs them in the menu engine otherwise.
use super::*;
use core::ops::ControlFlow;
use horizon_client::menu::Input;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// A left press this mod took: its release clicks nothing.
static PRESSED: AtomicBool = AtomicBool::new(false);
/// Wheel steps not yet acted on (a trackpad scrolls in fractions).
static WHEEL: Mutex<f32> = Mutex::new(0.0);
/// ToolPicker.lastToolPickSlot: the search starts after the last pick, so equal tools take turns.
static LAST_PICK: AtomicUsize = AtomicUsize::new(usize::MAX);

struct Mods {
    shift: bool,
    ctrl: bool,
    alt: bool,
    space: bool,
}

fn mods(game: &Game<'_>) -> Mods {
    Mods {
        shift: game.has_shift_down(),
        ctrl: game.has_control_down(),
        alt: game.modifiers.alt_key(),
        space: game.physical_keys.contains(&KeyCode::Space),
    }
}

/// The open screen is one whose clicks are vanilla's plain ones: the tweaks' own list.
fn on(game: &Game<'_>) -> bool {
    if game.screen != Screen::Inventory || game.station.is_some() || game.mount_active() || game.dialog_open() {
        return false;
    }
    if game.remote.is_some() {
        game.remote_menu_type().is_none_or(super::tweaks::tweak_menu)
    } else {
        matches!(game.container_kind(), None | Some(BlockEntity::Chest(_)))
    }
}

/// InventoryScreen (PlayerInventoryFocusedContainerScreenHelper) rather than a container's screen.
fn player_screen(game: &Game<'_>) -> bool {
    game.craft_size == 2 && !game.slots().iter().any(|s| (300..500).contains(s))
}

fn furnace(game: &Game<'_>) -> bool {
    game.remote_menu_type()
        .is_some_and(|n| n.ends_with("furnace") || n == "minecraft:smoker")
}

fn is_result(game: &mut Game<'_>, slot: usize) -> bool {
    if slot == 200 {
        return true;
    }
    let mut result = false;
    game.menu_session(|menu, _, to_menu| {
        result = to_menu(slot).is_some_and(|i| menu.is_result(i));
        Some(false)
    });
    result
}

/// ContainerScreenHelper.getScope. Player inventory 0 (the hotbar -1 when `prefer` smaller scopes,
/// soft hotbar scoping), the other side 1, None where nothing can be put (a result, a furnace's fuel,
/// armour). The inventory screen: hotbar 0, main inventory 1, armour 2, off hand -1, crafting 3.
fn scope(game: &mut Game<'_>, slot: usize, prefer: bool) -> Option<i32> {
    if player_screen(game) {
        return match slot {
            0..9 => Some(0),
            9..36 => Some(1),
            500..504 => Some(2),
            504 => Some(-1),
            100..109 | 200 => Some(3),
            _ => None,
        };
    }
    match slot {
        0..9 => Some(if prefer { -1 } else { 0 }),
        9..36 => Some(0),
        100..109 => Some(1),
        300..500 if !(furnace(game) && slot == 301) && !is_result(game, slot) => Some(1),
        _ => None,
    }
}

fn lower(scope: i32) -> bool {
    scope <= 0
}

fn complementary(scope: i32) -> i32 {
    if lower(scope) { 1 } else { 0 }
}

fn in_scope(game: &mut Game<'_>, scope_id: i32, prefer: bool) -> Vec<usize> {
    let slots = game.slots();
    slots
        .into_iter()
        .filter(|&s| scope(game, s, prefer) == Some(scope_id))
        .collect()
}

/// ItemStackUtils.areItemsOfSameKind, mode SOME: the same item, its components equal but for damage
/// and enchantments.
fn same_kind(a: &ItemStack, b: &ItemStack) -> bool {
    if a.item != b.item {
        return false;
    }
    let strip = |s: &ItemStack| {
        s.extra.as_deref().and_then(|d| {
            let mut d = d.clone();
            d.enchantments.clear();
            (d != Default::default()).then_some(d)
        })
    };
    strip(a) == strip(b)
}

fn click(game: &mut Game<'_>, slot: usize, button: i32, input: Input) -> bool {
    game.tweak_click(slot, button, input)
}

fn send_stack(game: &mut Game<'_>, slot: usize) {
    if game.slot_view(slot).is_some() {
        click(game, slot, 0, Input::QuickMove);
    }
}

/// One item where a Shift click sends the stack: pick it up, put one back, quick move that one,
/// put the rest back.
fn send_single(game: &mut Game<'_>, slot: usize) {
    let Some(stack) = game.slot_view(slot) else {
        return;
    };
    if stack.count == 1 {
        click(game, slot, 0, Input::QuickMove);
        return;
    }
    if click(game, slot, 0, Input::Pickup) {
        click(game, slot, 1, Input::Pickup);
        click(game, slot, 0, Input::QuickMove);
        click(game, slot, 0, Input::Pickup);
    }
}

fn send_all_of_kind(game: &mut Game<'_>, reference: usize) {
    let Some(stack) = game.slot_view(reference) else {
        return;
    };
    let Some(sc) = scope(game, reference, false) else {
        return;
    };
    for s in in_scope(game, sc, false) {
        if game.slot_view(s).is_some_and(|v| same_kind(&v, &stack)) {
            send_stack(game, s);
        }
    }
}

fn send_all_from(game: &mut Game<'_>, reference: usize) {
    let Some(sc) = scope(game, reference, true) else {
        return;
    };
    for s in in_scope(game, sc, true) {
        send_stack(game, s);
    }
}

/// Every stack in `sc` whose kind the other side holds goes over.
fn deposit_all_from(game: &mut Game<'_>, sc: i32) {
    let kinds: Vec<ItemStack> = in_scope(game, complementary(sc), false)
        .into_iter()
        .filter_map(|s| game.slot_view(s))
        .collect();
    for s in in_scope(game, sc, false) {
        if game.slot_view(s).is_some_and(|v| kinds.iter().any(|k| same_kind(k, &v))) {
            send_stack(game, s);
        }
    }
}

/// ContainerScreenHelper.restockAllOfAKind(Iterator, int): fills `targets` up from the other side,
/// taking from its last slots first.
fn restock(game: &mut Game<'_>, targets: &[usize], comp: i32) {
    let takes: Vec<usize> = game.slots().into_iter().rev().collect();
    let mut takes = takes.into_iter();
    let mut take: Option<usize> = None;
    let mut taking = 0i64;
    for &target in targets {
        let Some(stack) = game.slot_view(target) else {
            continue;
        };
        let mut space = i64::from(game.max_stack(&stack)) - i64::from(stack.count);
        while space > 0 {
            if taking == 0 {
                loop {
                    let Some(t) = takes.next() else {
                        return;
                    };
                    if scope(game, t, false) != Some(comp) {
                        continue;
                    }
                    if let Some(v) = game.slot_view(t)
                        && same_kind(&v, &stack)
                    {
                        taking = i64::from(v.count);
                        take = Some(t);
                        click(game, t, 0, Input::Pickup);
                        break;
                    }
                }
            }
            click(game, target, 0, Input::Pickup);
            space -= taking;
            taking = if space <= 0 { -space } else { 0 };
        }
    }
    if taking > 0
        && let Some(t) = take
    {
        click(game, t, 0, Input::Pickup);
    }
}

fn restock_all_of_kind(game: &mut Game<'_>, reference: usize) {
    let Some(stack) = game.slot_view(reference) else {
        return;
    };
    let Some(sc) = scope(game, reference, true) else {
        return;
    };
    let targets: Vec<usize> = in_scope(game, sc, true)
        .into_iter()
        .filter(|&s| game.slot_view(s).is_some_and(|v| same_kind(&v, &stack)))
        .collect();
    restock(game, &targets, complementary(sc));
}

fn restock_all(game: &mut Game<'_>, sc: i32) {
    let mut groups: Vec<(ItemStack, Vec<usize>)> = Vec::new();
    for s in in_scope(game, sc, false) {
        let Some(v) = game.slot_view(s) else {
            continue;
        };
        if v.count >= game.max_stack(&v) {
            continue;
        }
        match groups.iter_mut().find(|(k, _)| same_kind(k, &v)) {
            Some((_, g)) => g.push(s),
            None => groups.push((v, vec![s])),
        }
    }
    let comp = complementary(sc);
    for (_, g) in groups {
        restock(game, &g, comp);
    }
}

fn drop_stack(game: &mut Game<'_>, slot: usize) {
    if game.slot_view(slot).is_some() {
        click(game, slot, 1, Input::Throw);
    }
}

fn drop_all_of_kind(game: &mut Game<'_>, reference: usize) {
    let Some(stack) = game.slot_view(reference) else {
        return;
    };
    let Some(sc) = scope(game, reference, false) else {
        return;
    };
    for s in in_scope(game, sc, false) {
        if game.slot_view(s).is_some_and(|v| same_kind(&v, &stack)) {
            drop_stack(game, s);
        }
    }
}

fn drop_all_from(game: &mut Game<'_>, reference: usize) {
    let Some(sc) = scope(game, reference, true) else {
        return;
    };
    for s in in_scope(game, sc, true) {
        drop_stack(game, s);
    }
}

/// The armour slot an item is worn in (500 head .. 503 feet).
fn armour_slot(game: &Game<'_>, stack: &ItemStack) -> Option<usize> {
    match game.items().get(&stack.item)?.equip_slot.as_deref()? {
        "head" => Some(500),
        "chest" => Some(501),
        "legs" => Some(502),
        "feet" => Some(503),
        _ => None,
    }
}

/// ContainerScreenHelper.scroll. `up` is the wheel turned away from the player.
fn scroll_slot(game: &mut Game<'_>, slot: usize, up: bool) {
    let m = mods(game);
    let Some(stack) = game.slot_view(slot) else {
        return;
    };
    // MixinAbstractContainerScreen: the wheel up on armour in the inventory screen swaps it on.
    if up
        && player_screen(game)
        && !(500..504).contains(&slot)
        && let Some(armour) = armour_slot(game, &stack)
    {
        if click(game, slot, 0, Input::Pickup) {
            click(game, armour, 0, Input::Pickup);
            click(game, slot, 0, Input::Pickup);
        }
        return;
    }
    let Some(sc) = scope(game, slot, false) else {
        // Nothing can be put here (a result, a furnace's fuel): the wheel down takes the stack out
        // as a Shift click does.
        if !up {
            send_stack(game, slot);
        }
        return;
    };
    if is_result(game, slot) {
        if !up {
            send_stack(game, slot);
        }
        return;
    }
    if lower(sc) == up {
        // scrollSend
        if m.space {
            deposit_all_from(game, sc);
        } else if m.ctrl {
            send_all_of_kind(game, slot);
        } else if m.shift {
            send_stack(game, slot);
        } else {
            send_single(game, slot);
        }
        return;
    }
    // scrollReceive
    if m.space {
        if m.shift {
            restock_all(game, sc);
        } else {
            restock_all_of_kind(game, slot);
        }
        return;
    }
    let slots = game.slots();
    if !m.shift && !m.ctrl {
        let mut best: Option<(usize, u32)> = None;
        for s in slots {
            let Some(ss) = scope(game, s, false) else {
                continue;
            };
            if ss == sc || lower(ss) != up || is_result(game, s) {
                continue;
            }
            if let Some(v) = game.slot_view(s)
                && same_kind(&v, &stack)
                && best.is_none_or(|(_, n)| v.count < n)
            {
                best = Some((s, v.count));
                if v.count == 1 {
                    break;
                }
            }
        }
        if let Some((s, _)) = best {
            send_single(game, s);
        }
    } else {
        for s in slots {
            let Some(ss) = scope(game, s, false) else {
                continue;
            };
            if ss != sc && game.slot_view(s).is_some_and(|v| same_kind(&v, &stack)) {
                send_stack(game, s);
                if !m.ctrl {
                    break;
                }
            }
        }
    }
}

pub(super) fn scroll(game: &mut Game<'_>, delta: f32, slot: Option<usize>) -> ControlFlow<bool> {
    if !on(game) || game.carried.is_some() {
        return ControlFlow::Continue(());
    }
    let Some(slot) = slot else {
        *WHEEL.lock().unwrap() = 0.0;
        return ControlFlow::Continue(());
    };
    if game.slot_view(slot).is_none() {
        return ControlFlow::Continue(());
    }
    let steps = {
        let mut wheel = WHEEL.lock().unwrap();
        *wheel += delta;
        let steps = wheel.trunc();
        *wheel -= steps;
        steps
    };
    for _ in 0..steps.abs() as u32 {
        scroll_slot(game, slot, steps > 0.0);
    }
    ControlFlow::Break(true)
}

pub(super) fn press(game: &mut Game<'_>, right: bool) -> ControlFlow<()> {
    if right || !on(game) || game.carried.is_some() {
        return ControlFlow::Continue(());
    }
    let (Some(slot), false) = game.inventory_target() else {
        return ControlFlow::Continue(());
    };
    let m = mods(game);
    if m.alt {
        if m.ctrl {
            if m.shift {
                drop_all_from(game, slot);
            } else {
                drop_all_of_kind(game, slot);
            }
        } else {
            drop_stack(game, slot);
        }
    } else if m.ctrl {
        if m.shift {
            send_all_from(game, slot);
        } else {
            send_all_of_kind(game, slot);
        }
    } else if m.space {
        if let Some(sc) = scope(game, slot, false) {
            deposit_all_from(game, sc);
        }
    } else {
        return ControlFlow::Continue(());
    }
    PRESSED.store(true, Ordering::Relaxed);
    ControlFlow::Break(())
}

pub(super) fn release(game: &mut Game<'_>, right: bool) -> ControlFlow<()> {
    if right || !PRESSED.swap(false, Ordering::Relaxed) {
        return ControlFlow::Continue(());
    }
    game.tweak_release();
    ControlFlow::Break(())
}

pub(super) fn drag(game: &mut Game<'_>) -> ControlFlow<()> {
    if !game.physical_buttons.contains(&MouseButton::Left)
        || game.carried.is_some()
        || game.inv_drag.is_some()
        || !on(game)
    {
        return ControlFlow::Continue(());
    }
    let m = mods(game);
    if !(m.alt || m.shift || m.ctrl) {
        return ControlFlow::Continue(());
    }
    let Some(slot) = game.inventory_target().0 else {
        return ControlFlow::Continue(());
    };
    if game.slot_view(slot).is_none() {
        return ControlFlow::Continue(());
    }
    if m.alt {
        drop_stack(game, slot);
    } else if m.shift {
        send_stack(game, slot);
    } else {
        send_all_of_kind(game, slot);
    }
    ControlFlow::Break(())
}

#[derive(Clone, Copy)]
enum SortMode {
    Creative,
    Quantity,
    Alphabet,
}

/// InventorySorter: the part under `slot` merged and sorted with `inv_sort::plan`'s left clicks.
/// Only parts that are plain storage: the inventory's (hotbar and main apart) or a chest's.
fn sort(game: &mut Game<'_>, slot: usize, mode: SortMode) -> bool {
    let Some(sc) = scope(game, slot, true) else {
        return false;
    };
    let furnace = furnace(game);
    let slots = in_scope(game, sc, true);
    if slots.len() <= 1 || !slots.iter().all(|&s| s < 36 || ((300..500).contains(&s) && !furnace)) {
        return false;
    }
    let views: Vec<Option<ItemStack>> = slots.iter().map(|&s| game.slot_view(s)).collect();
    let clicks = {
        let names = |id: &str| game.pack.names.display_item(id);
        let name = |s: &ItemStack| horizon_client::stations::hover_name(&names, s);
        let order = |s: &ItemStack| game.creative_screen.order_of(s).unwrap_or(usize::MAX);
        let mut totals: Vec<(std::sync::Arc<str>, u64)> = Vec::new();
        for v in views.iter().flatten() {
            match totals.iter_mut().find(|(i, _)| *i == v.item) {
                Some((_, n)) => *n += u64::from(v.count),
                None => totals.push((v.item.clone(), u64::from(v.count))),
            }
        }
        let rank = |s: &ItemStack| -> horizon_client::inv_sort::Rank {
            match mode {
                SortMode::Creative => (order(s), name(s)),
                SortMode::Alphabet => (0, name(s).to_lowercase()),
                SortMode::Quantity => {
                    let total = totals.iter().find(|(i, _)| *i == s.item).map_or(0, |(_, n)| *n);
                    (usize::MAX - total as usize, format!("{:020}{}", order(s), name(s)))
                }
            }
        };
        let max = |s: &ItemStack| game.max_stack(s);
        horizon_client::inv_sort::plan(&views, &rank, &max)
    };
    for i in clicks {
        click(game, slots[i], 0, Input::Pickup);
    }
    true
}

pub(super) fn sort_press(game: &mut Game<'_>, button: MouseButton) -> ControlFlow<bool> {
    if button != MouseButton::Middle || !on(game) || game.carried.is_some() {
        return ControlFlow::Continue(());
    }
    let Some(slot) = game.inventory_target().0 else {
        return ControlFlow::Continue(());
    };
    // mouseWheelie_triggerSort: in creative the middle button on a stack clones it.
    if game.creative && game.slot_view(slot).is_some() {
        return ControlFlow::Continue(());
    }
    let m = mods(game);
    let mode = if m.shift {
        SortMode::Quantity
    } else if m.ctrl {
        SortMode::Alphabet
    } else {
        SortMode::Creative
    };
    if sort(game, slot, mode) {
        ControlFlow::Break(true)
    } else {
        ControlFlow::Continue(())
    }
}

fn in_tags(game: &Game<'_>, stack: &ItemStack, tags: &[&str]) -> bool {
    let items = game.items();
    tags.iter().any(|t| items.item_in_tag(t, &stack.item))
}

/// MWClient.isTool: a mining tool or shears.
fn is_tool(game: &Game<'_>, stack: &ItemStack) -> bool {
    &*stack.item == "minecraft:shears"
        || in_tags(game, stack, &["minecraft:pickaxes", "minecraft:axes", "minecraft:shovels", "minecraft:hoes"])
}

/// MWClient.isWeapon: a sword, a trident or a ranged weapon.
fn is_weapon(game: &Game<'_>, stack: &ItemStack) -> bool {
    matches!(&*stack.item, "minecraft:trident" | "minecraft:bow" | "minecraft:crossbow")
        || in_tags(game, stack, &["minecraft:swords"])
}

/// ToolPicker.findToolFor: the first slot (after the last pick) correct for the block's drops, else
/// the fastest; the selected one only when nothing else is better than a hand.
fn find_tool(game: &Game<'_>, block: &str) -> Option<usize> {
    let items = game.items();
    let size = 36.min(game.inventory.len());
    let selected = game.selected;
    let last = LAST_PICK.load(Ordering::Relaxed);
    let last = if last == usize::MAX { 0 } else { last };
    let (mut best, mut best_speed) = (None, 1.0f32);
    for i in 1..=size {
        let index = (i + last) % size;
        if index == selected {
            continue;
        }
        let stack = game.inventory[index].as_ref();
        if stack.is_none() {
            continue;
        }
        if items.correct_for_drops(stack, block) {
            return Some(index);
        }
        let speed = items.mining_speed(stack, block);
        if speed > best_speed {
            best = Some(index);
            best_speed = speed;
        }
    }
    if best.is_none() {
        let held = game.inventory[selected].as_ref();
        if items.correct_for_drops(held, block) || items.mining_speed(held, block) > 1.0 {
            return Some(selected);
        }
    }
    best
}

fn find_weapon(game: &Game<'_>) -> Option<usize> {
    let size = 36.min(game.inventory.len());
    let last = LAST_PICK.load(Ordering::Relaxed);
    let last = if last == usize::MAX { 0 } else { last };
    (1..=size)
        .map(|i| (i + last) % size)
        .find(|&i| i != game.selected && game.inventory[i].as_ref().is_some_and(|s| is_weapon(game, s)))
}

/// ToolPicker.pick: a hotbar slot is selected; a main inventory one swaps into the hotbar
/// (StackPicker, PREFER_EMPTY_HOTBAR_SLOTS) and is selected.
fn pick_slot(game: &mut Game<'_>, index: usize) {
    LAST_PICK.store(index, Ordering::Relaxed);
    if index < 9 {
        game.selected = index;
        return;
    }
    let target = (0..9)
        .map(|i| (game.selected + i) % 9)
        .find(|&i| game.inventory[i].is_none())
        .unwrap_or(game.selected);
    if game.tweak_click(index, target as i32, Input::Swap) {
        game.selected = target;
    }
}

pub(super) fn pick(game: &mut Game<'_>, button: i32) -> ControlFlow<Option<bool>> {
    if button != 2 || game.screen != Screen::Playing {
        return ControlFlow::Continue(());
    }
    let Some(held) = game.inventory.get(game.selected).cloned().flatten() else {
        return ControlFlow::Continue(());
    };
    if !is_tool(game, &held) && !is_weapon(game, &held) {
        return ControlFlow::Continue(());
    }
    let index = match game.pick_entity_hit() {
        Some(Some(_)) => find_weapon(game),
        Some(None) => None,
        None => match game.pick() {
            Some(hit) => {
                let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
                let name = game.res.facts().get(state).name.clone();
                find_tool(game, &name)
            }
            None => find_weapon(game),
        },
    };
    match index {
        Some(i) if i != game.selected => {
            pick_slot(game, i);
            ControlFlow::Break(Some(false))
        }
        _ => {
            LAST_PICK.store(index.unwrap_or(usize::MAX), Ordering::Relaxed);
            ControlFlow::Continue(())
        }
    }
}
