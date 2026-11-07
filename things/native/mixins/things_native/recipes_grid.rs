// The grid find_with was given, for the sock recipes hook.
fn things_recipes_grid(cells: &[Option<ItemStack>], width: usize) {
    crate::crafting::things::stash(cells, width);
}
