// A sock upgrade's potion leaves a glass bottle.
fn things_recipes_remainders(
    value: Vec<Option<ItemStack>>,
    recipe: &str,
    cells: &[Option<ItemStack>],
    width: usize,
) -> Vec<Option<ItemStack>> {
    crate::crafting::things::remainders(value, recipe, cells, width)
}
