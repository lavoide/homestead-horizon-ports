// SockUpgradeRecipe and JumpySocksRecipe, before the recipe files.
fn things_recipes_special(recipes: &Recipes) -> core::ops::ControlFlow<Option<(&str, ItemStack)>> {
    let _ = recipes;
    match crate::crafting::things::special() {
        Some(found) => core::ops::ControlFlow::Break(Some(found)),
        None => core::ops::ControlFlow::Continue(()),
    }
}
