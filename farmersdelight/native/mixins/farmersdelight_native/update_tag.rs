// What clients get of the stove, skillet and board: their items.
fn fd_update_tag(entity: &ModBlockEntity, tag: &mut Option<Compound>) {
    if tag.is_none() {
        crate::game::farmersdelight::update_tag(entity, tag);
    }
}
