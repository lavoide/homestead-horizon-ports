// AnvilScreenHandlerMixin.setOutput: the Hardening Catalyst makes a damageable item unbreakable
// (not one already unbreakable; Homestead's hardening_catalyst_blacklist tag is empty) for 30
// levels, the catalyst used up, the name as the rename box has it.
fn things_catalyst(items: &Items, input: Option<&ItemStack>, addition: Option<&ItemStack>, name: Option<&str>) -> core::ops::ControlFlow<AnvilResult> {
    let (Some(input), Some(addition)) = (input, addition) else {
        return core::ops::ControlFlow::Continue(());
    };
    if &*addition.item != "things:hardening_catalyst" {
        return core::ops::ControlFlow::Continue(());
    }
    let unbreakable = input.data().is_some_and(|d| d.other.iter().any(|(k, _)| k == "minecraft:unbreakable"));
    if !items.get(&input.item).is_some_and(|i| i.max_damage > 0) || unbreakable {
        return core::ops::ControlFlow::Break(AnvilResult::default());
    }
    let mut out = input.clone();
    out.data_mut().other.push((
        "minecraft:unbreakable".to_owned(),
        horizon_world::nbt::Tag::Compound(Default::default()),
    ));
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => out.data_mut().custom_name = Some(horizon_world::nbt::Tag::String(n.into())),
        None => out.data_mut().custom_name = None,
    }
    core::ops::ControlFlow::Break(AnvilResult {
        result: Some(out),
        cost: 30,
        repair_item_count_cost: 1,
        only_renaming: false,
    })
}
