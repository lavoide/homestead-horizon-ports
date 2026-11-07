// onTakeResult: the levels paid (Player.giveExperienceLevels(-cost), as anvil_take), then the
// inputs as the operation leaves them. No experience and no grindstone sound, as the original's
// cancelled onTake.
fn grind_take(menu: &mut Menu, env: &Env, s: usize) -> core::ops::ControlFlow<()> {
    if menu.kind != Kind::Grindstone || menu.slots[s].role != Role::GrindResult {
        return core::ops::ControlFlow::Continue(());
    }
    let (a, b) = (menu.containers[MAIN][0].clone(), menu.containers[MAIN][1].clone());
    let Some(cost) = crate::stations::grindenchantments::cost(env.enchantments, a.as_ref(), b.as_ref())
    else {
        return core::ops::ControlFlow::Continue(());
    };
    let Some([new_a, new_b]) = crate::stations::grindenchantments::take(a.as_ref(), b.as_ref()) else {
        return core::ops::ControlFlow::Continue(());
    };
    if !menu.creative {
        menu.level_change -= cost;
        menu.drops.push(format!("levels{}{}", if cost <= 0 { "+" } else { "" }, -cost));
    }
    menu.put(env, 0, new_a);
    menu.put(env, 1, new_b);
    core::ops::ControlFlow::Break(())
}
