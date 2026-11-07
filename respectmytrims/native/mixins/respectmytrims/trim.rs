// The Java mod checks the trim material's asset name, "gold": vanilla's `minecraft:gold`.
fn gold_trim(game: &Game<'_>) -> core::ops::ControlFlow<bool> {
    let trimmed = game.armor.iter().flatten().any(|s| {
        s.data()
            .and_then(|d| d.trim.as_ref())
            .is_some_and(|(material, _)| &**material == "minecraft:gold")
    });
    if trimmed {
        core::ops::ControlFlow::Break(true)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
