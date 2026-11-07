// The worn carved face's blur.
fn mcaccel_overlay(mut value: hud_overlays::CameraOverlays, game: &Game<'_>) -> hud_overlays::CameraOverlays {
    if let Some(blur) = crate::game::mcaccel::blur(game) {
        value.equipment = vec![blur];
    }
    value
}
