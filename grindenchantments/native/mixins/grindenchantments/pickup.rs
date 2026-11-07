// canTakeResult: creative, or at least the cost in levels.
fn grind_pickup(menu: &Menu, env: &Env, s: usize) -> core::ops::ControlFlow<bool> {
    if menu.kind != Kind::Grindstone || menu.slots[s].role != Role::GrindResult {
        return core::ops::ControlFlow::Continue(());
    }
    let (a, b) = (menu.containers[MAIN][0].as_ref(), menu.containers[MAIN][1].as_ref());
    match crate::stations::grindenchantments::cost(env.enchantments, a, b) {
        Some(cost) => core::ops::ControlFlow::Break(menu.creative || menu.levels >= cost),
        None => core::ops::ControlFlow::Continue(()),
    }
}
