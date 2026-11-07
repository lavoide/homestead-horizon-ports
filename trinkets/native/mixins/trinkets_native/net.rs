// The Trinkets key's message, on the server.
fn trinkets_net(game: &mut Game<'_>, player: u128, channel: &str) {
    crate::game::trinkets::net(game, player, channel);
}
