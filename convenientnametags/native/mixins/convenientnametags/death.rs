// removeNameAndNameTag on death: a named mob's name drops as a tag.
fn named_death(game: &mut Game<'_>, m: &Mob) {
    if let Some(name) = m.custom_name.clone() {
        crate::game::creatures::convenientnametags::drop_tag(game, m, &name);
    }
}
