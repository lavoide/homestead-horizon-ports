// The trinket slots an item fits.
fn trinkets_tooltip(stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    crate::game::trinkets::tooltip(stack, lines);
}
