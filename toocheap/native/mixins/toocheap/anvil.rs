fn anvil_result(
    e: &Enchantments,
    items: &Items,
    input: Option<&ItemStack>,
    addition: Option<&ItemStack>,
    name: Option<&str>,
    hover_name: &str,
    creative: bool,
) -> core::ops::ControlFlow<AnvilResult> {
    core::ops::ControlFlow::Break(crate::stations::toocheap::anvil_result(
        e, items, input, addition, name, hover_name, creative,
    ))
}
