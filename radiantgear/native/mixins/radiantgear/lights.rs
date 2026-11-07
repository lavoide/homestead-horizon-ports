// Radiant Gear: the worn trinkets' light.
fn radiantgear_lights(value: Vec<(Source, Option<String>)>, game: &Game<'_>, alpha: f64) -> Vec<(Source, Option<String>)> {
    crate::game::radiantgear::worn_lights(game, value, alpha)
}
