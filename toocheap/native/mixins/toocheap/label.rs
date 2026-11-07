// "Too Expensive!" becomes "Enchantment Cost: N" with the anvil's cost.
fn cost_label(game: &Game<'_>, key: &str) -> core::ops::ControlFlow<String> {
    if key != "container.repair.expensive" {
        return core::ops::ControlFlow::Continue(());
    }
    let Some(station) = &game.station else { return core::ops::ControlFlow::Continue(()) };
    let line = horizon_client::lang::get_or("container.repair.cost", "Enchantment Cost: %1$s")
        .replace("%1$s", &station.anvil.cost.to_string());
    core::ops::ControlFlow::Break(line)
}
