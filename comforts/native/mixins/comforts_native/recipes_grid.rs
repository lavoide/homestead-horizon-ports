// The grid find_with was given, for the hook after this one.
fn comforts_recipes_grid(cells: &[Option<ItemStack>], width: usize) {
    crate::crafting::comforts::stash(cells, width);
}
