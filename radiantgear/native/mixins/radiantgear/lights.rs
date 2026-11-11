// Radiant Gear: the worn trinkets' light, chosen with the other dynamic lights.
fn radiantgear_lights(game: &mut Game<'_>, camera: glam::DVec3, alpha: f64) -> core::ops::ControlFlow<()> {
    crate::game::radiantgear::gather_with_worn(game, camera, alpha)
}
