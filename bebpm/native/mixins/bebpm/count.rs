// Count the power providers in the box around the table (not the table itself).
fn count(game: &Game<'_>, pos: [i32; 3]) -> core::ops::ControlFlow<i32> {
    const R: i32 = 10;
    let Some(tag) = game.items().block_tags.get("minecraft:enchantment_power_provider") else {
        return core::ops::ControlFlow::Break(0);
    };
    let facts = game.res.facts();
    let mut n = 0;
    for x in -R..=R {
        for y in -R..=R {
            for z in -R..=R {
                if x == 0 && y == 0 && z == 0 {
                    continue;
                }
                let name = &facts.get(game.world.block(pos[0] + x, pos[1] + y, pos[2] + z)).name;
                if tag.contains(&**name) {
                    n += 1;
                }
            }
        }
    }
    core::ops::ControlFlow::Break(n)
}
