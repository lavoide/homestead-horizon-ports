// onUpdateResult: the operation's result, before vanilla's.
fn grind_result(
    e: &Enchantments,
    input: Option<&ItemStack>,
    additional: Option<&ItemStack>,
) -> core::ops::ControlFlow<Option<ItemStack>> {
    match crate::stations::grindenchantments::result(e, input, additional) {
        Some(result) => core::ops::ControlFlow::Break(result),
        None => core::ops::ControlFlow::Continue(()),
    }
}
