// GrindstoneScreenMixin: the station drawn as always (draw_station re-entered behind a guard),
// then "Enchantment Cost: N" where the anvil puts its cost, green, or red when the player can't
// pay.
//
// Injected as `crate::game::stations::grindenchantments`.

use super::{Game, Kind, argb};
use horizon_client::{enchant::Enchantments, hud::HudBatch, items::ItemStack};

// A copy of runtime.rs's `op` and `cost` (the lib's mixin module is crate-private).
const BOOK: &str = "minecraft:book";
const ENCHANTED_BOOK: &str = "minecraft:enchanted_book";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// The enchanted item is in this input slot, the book in the other.
    Disenchant(usize),
    /// Slot 0 an enchanted book, slot 1 a book or an enchanted book.
    Move,
}

fn is(stack: Option<&ItemStack>, item: &str) -> bool {
    stack.is_some_and(|s| &*s.item == item)
}

/// `ItemStack.hasEnchantments` (1.20.1): the `Enchantments` list, which an enchanted book lacks.
fn enchanted(stack: Option<&ItemStack>) -> bool {
    stack.is_some_and(|s| !s.enchantments().is_empty())
}

/// `isDisenchantOperation`, then `isMoveOperation`.
pub fn op(input: Option<&ItemStack>, additional: Option<&ItemStack>) -> Option<Op> {
    if enchanted(input) && is(additional, BOOK) {
        return Some(Op::Disenchant(0));
    }
    if enchanted(additional) && is(input, BOOK) {
        return Some(Op::Disenchant(1));
    }
    if is(input, ENCHANTED_BOOK) && (is(additional, ENCHANTED_BOOK) || is(additional, BOOK)) {
        return Some(Op::Move);
    }
    None
}

fn slots<'a>(
    op: Op,
    input: Option<&'a ItemStack>,
    additional: Option<&'a ItemStack>,
) -> (Option<&'a ItemStack>, Option<&'a ItemStack>) {
    match op {
        Op::Disenchant(1) => (additional, input),
        _ => (input, additional),
    }
}

/// `getLevelCost`: `ceil` of the operation's cost function.
pub fn cost(e: &Enchantments, input: Option<&ItemStack>, additional: Option<&ItemStack>) -> Option<i32> {
    let op = op(input, additional)?;
    let from = slots(op, input, additional).0?;
    let raw = match op {
        // transform(count_min_power, factor 0.3, offset 8).
        Op::Disenchant(_) => {
            let power: f64 = from
                .enchantments()
                .iter()
                .map(|(id, l)| f64::from(e.get(id).map_or(0, |d| d.min_cost.calculate(*l))))
                .sum();
            power * 0.3 + 8.0
        }
        // transform(first_enchantment(count_levels(normal 3, treasure 8)), factor 0.5, offset 0.5).
        Op::Move => {
            let first = from.stored_enchantments().first().map_or(1.0, |(id, l)| {
                let factor = if e.in_tag("minecraft:treasure", id) { 8.0 } else { 3.0 };
                f64::from(*l) * factor
            });
            first * 0.5 + 0.5
        }
    };
    Some(raw.ceil() as i32)
}


thread_local! {
    static DRAWING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn draw(game: &mut Game<'_>, batch: &mut HudBatch) -> Option<bool> {
    if DRAWING.with(|d| d.get()) {
        return None;
    }
    DRAWING.with(|d| d.set(true));
    let shown = game.draw_station(batch);
    DRAWING.with(|d| d.set(false));
    let cost = game.station.as_ref().and_then(|s| {
        if s.kind != Kind::Grindstone || s.result.is_none() {
            return None;
        }
        cost(
            &game.stations().enchantments,
            s.inputs[0].as_ref(),
            s.inputs[1].as_ref(),
        )
    });
    let Some(cost) = cost.filter(|_| shown) else {
        return Some(shown);
    };
    let color = if game.creative || game.experience.level >= cost {
        argb(-8323296)
    } else {
        argb(-40864)
    };
    let line = game
        .translate("container.repair.cost", "Enchantment Cost: %1$s")
        .replace("%1$s", &cost.to_string());
    let (ox, oy) = game.inventory_origin(batch);
    let atlas = &game.gui.atlas;
    let tw = HudBatch::text_width(atlas, &line);
    let tx = 176.0 - 8.0 - tw - 2.0;
    batch.fill(atlas, ox + tx - 2.0, oy + 67.0, 176.0 - 8.0 - (tx - 2.0), 12.0, argb(1325400064));
    batch.text(atlas, &line, ox + tx, oy + 69.0, color, true);
    Some(shown)
}
