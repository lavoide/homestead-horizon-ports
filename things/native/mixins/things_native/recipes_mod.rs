// Things' special crafting recipes, SockUpgradeRecipe and JumpySocksRecipe.
// Injected as `crate::crafting::things` (horizon_client_lib).
use super::*;

pub(crate) const UPGRADE_RECIPE: &str = "things:sock_upgrade_crafting";
pub(crate) const JUMPY_RECIPE: &str = "things:jumpy_sock_crafting";

thread_local! {
    static GRID: std::cell::RefCell<(Vec<Option<ItemStack>>, usize)> =
        const { std::cell::RefCell::new((Vec::new(), 0)) };
}
pub(crate) fn stash(cells: &[Option<ItemStack>], width: usize) {
    GRID.with(|g| *g.borrow_mut() = (cells.to_vec(), width));
}

fn custom_int(stack: &ItemStack, key: &str) -> i32 {
    let Some(d) = stack.data() else { return 0 };
    let Some((_, tag)) = d.other.iter().find(|(k, _)| k == "minecraft:custom_data") else {
        return 0;
    };
    match tag.get(key) {
        Some(Tag::Int(v)) => *v,
        Some(Tag::Byte(v)) => i32::from(*v),
        _ => 0,
    }
}

fn with_custom(stack: &ItemStack, key: &str, value: Tag) -> ItemStack {
    let mut out = stack.clone();
    out.count = 1;
    let d = out.data_mut();
    let mut c = match d.other.iter().find(|(k, _)| k == "minecraft:custom_data") {
        Some((_, Tag::Compound(c))) => c.clone(),
        _ => Default::default(),
    };
    c.insert(key.into(), value);
    d.other.retain(|(k, _)| k != "minecraft:custom_data");
    d.other.push(("minecraft:custom_data".to_owned(), Tag::Compound(c)));
    let speed = custom_int(&out, "Speed");
    let jumpy = custom_int(&out, "Jumpy") != 0;
    let off = custom_int(&out, "JumpBoostDisabled") != 0;
    let mut lines = Vec::new();
    if jumpy {
        lines.push(format!("↑ Jumpy{}", if off { " (off: use to toggle)" } else { " (use to toggle)" }));
    }
    lines.push(match speed {
        0 => "☄ Average".to_owned(),
        1 => "☄ Speedy".to_owned(),
        2 => "☄ Usain Bolt".to_owned(),
        n => format!("☄ Naughty ({n})"),
    });
    lines.push("Works from your hotbar or off hand, one per kind (face, necklace, glove, shoes).".to_owned());
    let lore = Tag::List(
        lines
            .iter()
            .map(|l| {
                Tag::compound([
                    ("text", Tag::String(l.as_str().into())),
                    ("color", Tag::String("gray".into())),
                    ("italic", Tag::Byte(0)),
                ])
            })
            .collect(),
    );
    let d = out.data_mut();
    d.other.retain(|(k, _)| k != "minecraft:lore");
    d.other.push(("minecraft:lore".to_owned(), lore));
    out
}

/// matchOnce: exactly one stack in the grid passes.
fn once<'a>(filled: &[&'a ItemStack], test: impl Fn(&ItemStack) -> bool) -> Option<&'a ItemStack> {
    let mut found = filled.iter().filter(|s| test(s));
    let first = found.next()?;
    found.next().is_none().then_some(*first)
}

/// The sock recipe the stashed grid makes, if any (both need a grid of 2x2 or more).
pub(crate) fn special() -> Option<(&'static str, ItemStack)> {
    let (cells, width) = GRID.with(|g| g.borrow().clone());
    if width < 2 {
        return None;
    }
    let filled: Vec<&ItemStack> = cells.iter().flatten().filter(|s| s.count > 0).collect();
    if filled.len() != 3 {
        return None;
    }
    // SockUpgradeRecipe: gleaming powder, a strong swiftness potion and socks below Usain Bolt.
    if once(&filled, |s| &*s.item == "things:gleaming_powder").is_some()
        && once(&filled, |s| &*s.item == "minecraft:potion" && s.potion() == Some("minecraft:strong_swiftness")).is_some()
        && let Some(socks) = once(&filled, |s| &*s.item == "things:socks" && custom_int(s, "Speed") < 2)
    {
        return Some((UPGRADE_RECIPE, with_custom(socks, "Speed", Tag::Int(custom_int(socks, "Speed") + 1))));
    }
    // JumpySocksRecipe: gleaming compound, a rabbit foot charm and socks not yet jumpy.
    if once(&filled, |s| &*s.item == "things:gleaming_compound").is_some()
        && once(&filled, |s| &*s.item == "things:rabbit_foot_charm").is_some()
        && let Some(socks) = once(&filled, |s| &*s.item == "things:socks" && custom_int(s, "Jumpy") == 0)
    {
        return Some((JUMPY_RECIPE, with_custom(socks, "Jumpy", Tag::Byte(1))));
    }
    None
}

/// The upgrade's potion leaves a glass bottle (Things gives the potion that recipe remainder).
pub(crate) fn remainders(
    mut out: Vec<Option<ItemStack>>,
    recipe: &str,
    cells: &[Option<ItemStack>],
    width: usize,
) -> Vec<Option<ItemStack>> {
    let _ = width;
    if recipe != UPGRADE_RECIPE {
        return out;
    }
    for (i, c) in cells.iter().enumerate() {
        if c.as_ref().is_some_and(|s| &*s.item == "minecraft:potion") && i < out.len() {
            out[i] = Some(ItemStack::new("minecraft:glass_bottle", 1));
        }
    }
    out
}
