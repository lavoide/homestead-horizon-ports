// `bed`: the guest's bed is still there; a bag or hammock is one.
fn comforts_guest_bed(value: bool, game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> bool {
    value || crate::game::comforts::still_bed(game, state)
}
