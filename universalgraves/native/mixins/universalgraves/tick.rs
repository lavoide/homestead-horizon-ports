// Once a tick the graves are watched; each guest gets the grave messages meant for them.
fn ug_tick(game: &mut Game<'_>, host: &mut LanHost, conn: u32) {
    crate::game::universalgraves::watch(game);
    let Some(uuid) = host.hub.guests.iter().find(|g| g.conn == conn).map(|g| g.uuid) else {
        return;
    };
    for line in crate::game::universalgraves::take_messages(uuid) {
        let text = horizon_client::text::Text::literal(line);
        host.hub.send_to(
            conn,
            &C::SystemChat(play::SystemChat {
                content: horizon_client::lan::data::component_nbt(&text.component()),
                overlay: false,
            }),
        );
    }
}
