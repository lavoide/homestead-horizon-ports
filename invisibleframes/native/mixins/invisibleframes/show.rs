// The Java mod's ItemFrame.hurt HEAD: any hit makes the frame visible.
fn show(game: &mut Game<'_>) {
    if let Some((i, d)) = game.frame_hit()
        && game.before_block(d)
    {
        game.item_frames[i].invisible = false;
    }
}
