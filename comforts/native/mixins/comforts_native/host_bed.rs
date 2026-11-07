// `bed_gone`: not when it's a bag or hammock.
fn comforts_host_bed(value: bool, game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> bool {
    value && !crate::game::comforts::still_bed(game, state)
}
