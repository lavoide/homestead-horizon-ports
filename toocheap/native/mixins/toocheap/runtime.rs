// Too Cheap for Horizon: `stations::anvil_result` from Horizon 3fb4c31, with the 40-level cap
// taken out. Injected as `crate::stations::toocheap`; the bundle's OVERWRITE makes the original
// call this. Rebase it on a Horizon update (the compatibility pin makes the build say so).
use super::*;

pub fn anvil_result(
    e: &Enchantments,
    items: &Items,
    input: Option<&ItemStack>,
    addition: Option<&ItemStack>,
    name: Option<&str>,
    hover_name: &str,
    creative: bool,
) -> AnvilResult {
    let fail = AnvilResult::default();
    let Some(input) = input else { return fail };
    // EnchantmentHelper.canStoreEnchantments: enchantable or enchanted book... every
    // item with an enchantments component, which is all of them in 26.3.
    let mut result = input.clone();
    let mut enchantments: Levels = input.crafting_enchantments().to_vec();
    let mut price: i64 = 0;
    let tax: i64 =
        i64::from(input.repair_cost()) + i64::from(addition.map_or(0, ItemStack::repair_cost));
    let mut repair_item_count_cost = 0;
    let mut naming_cost = 0;
    if let Some(addition) = addition {
        // addition.has(STORED_ENCHANTMENTS): an enchanted book (its default
        // component), or any item given stored enchantments.
        let using_book = &*addition.item == "minecraft:enchanted_book"
            || !addition.stored_enchantments().is_empty();
        if is_damageable(items, &result) && is_valid_repair_item(e, items, input, addition) {
            let max = max_damage(items, &result);
            let mut repair = (result.damage as i32).min(max / 4);
            if repair <= 0 {
                return fail;
            }
            let mut count = 0;
            while repair > 0 && count < addition.count {
                result.damage = (result.damage as i32 - repair) as u32;
                price += 1;
                repair = (result.damage as i32).min(max / 4);
                count += 1;
            }
            repair_item_count_cost = count;
        } else {
            if !using_book && (result.item != addition.item || !is_damageable(items, &result)) {
                return fail;
            }
            if is_damageable(items, &result) && !using_book {
                let max = max_damage(items, &result);
                let remaining1 = max - input.damage as i32;
                let remaining2 = max_damage(items, addition) - addition.damage as i32;
                let additional = remaining2 + max * 12 / 100;
                let remaining = remaining1 + additional;
                let damage = (max - remaining).max(0);
                if damage < result.damage as i32 {
                    result.damage = damage as u32;
                    price += 2;
                }
            }
            let mut any_compatible = false;
            let mut any_incompatible = false;
            for (id, level) in addition.crafting_enchantments() {
                let current = enchantments
                    .iter()
                    .find(|(x, _)| x == id)
                    .map_or(0, |(_, l)| *l);
                let mut level = if current == *level {
                    level + 1
                } else {
                    (*level).max(current)
                };
                let Some(def) = e.get(id) else { continue };
                let mut compatible = e.can_enchant(items, id, &input.item);
                if creative || &*input.item == "minecraft:enchanted_book" {
                    compatible = true;
                }
                for (other, _) in &enchantments {
                    if other != id && !e.compatible(id, other) {
                        compatible = false;
                        price += 1;
                    }
                }
                if !compatible {
                    any_incompatible = true;
                } else {
                    any_compatible = true;
                    level = level.min(def.max_level);
                    item_data::set_level(&mut enchantments, id, level);
                    let mut fee = def.anvil_cost as i64;
                    if using_book {
                        fee = (fee / 2).max(1);
                    }
                    price += fee * i64::from(level);
                    if input.count > 1 {
                        price = 40;
                    }
                }
            }
            if any_incompatible && !any_compatible {
                return fail;
            }
        }
    }
    match name {
        Some(n) if !n.trim().is_empty() => {
            if n != hover_name {
                naming_cost = 1;
                price += 1;
                result.data_mut().custom_name = Some(horizon_world::nbt::Tag::String(n.into()));
            }
        }
        _ => {
            if input.custom_name().is_some() {
                naming_cost = 1;
                price += 1;
                result.data_mut().custom_name = None;
            }
        }
    }
    let mut cost = if price <= 0 {
        0
    } else {
        (tax + price).clamp(0, i64::from(i32::MAX)) as i32
    };
    let mut out = (price > 0).then_some(result);
    let mut only_renaming = false;
    if naming_cost == price && naming_cost > 0 {
        if cost >= 40 {
            cost = 39;
        }
        only_renaming = true;
    }
    // Too Cheap: no "Too Expensive!" cap (vanilla: `if cost >= 40 && !creative { out = None }`).
    if let Some(r) = out.as_mut() {
        let mut base = r
            .repair_cost()
            .max(addition.map_or(0, ItemStack::repair_cost));
        if naming_cost != price || naming_cost == 0 {
            base = increased_repair_cost(base);
        }
        r.data_mut().repair_cost = base;
        r.set_crafting_enchantments(enchantments);
        r.tidy();
    }
    AnvilResult {
        result: out,
        cost,
        repair_item_count_cost,
        only_renaming,
    }
}
