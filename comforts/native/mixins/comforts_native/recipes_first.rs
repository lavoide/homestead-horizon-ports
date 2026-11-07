// Comforts' crafting recipes before the others (a recipe id order, comforts: < minecraft:).
fn comforts_recipes_first(recipes: &Recipes) -> core::ops::ControlFlow<Option<(&str, ItemStack)>> {
    let (cells, width) = crate::crafting::comforts::grid();
    let Some(input) = Input::of(&cells, width) else {
        return core::ops::ControlFlow::Continue(());
    };
    let no_map = |_: i32| None;
    let ctx = Context { recipes, map_scale: &no_map };
    for (name, recipe) in &recipes.crafting {
        if !name.starts_with("comforts:") {
            continue;
        }
        if let Some(result) = Recipes::assemble_if_matches(&ctx, &input, recipe) {
            return core::ops::ControlFlow::Break(result.map(|r| (name.as_str(), r)));
        }
    }
    core::ops::ControlFlow::Continue(())
}
