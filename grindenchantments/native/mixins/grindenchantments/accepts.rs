// canInsert: a book goes in either slot.
fn accept_book(stack: &ItemStack) -> core::ops::ControlFlow<bool> {
    if &*stack.item == "minecraft:book" {
        core::ops::ControlFlow::Break(true)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
