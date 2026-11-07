//! Show Me What You Got 1.1.1 (Apace, MIT), the server's half: a `smwyg:share` message from a
//! player becomes the chat line `<name> text [item] text` for everyone (`chat.type.text`, as
//! ShowMeWhatYouGot's ServerMessageDecoratorEvent decorates it), written with `tellraw @a`.
use horizon_client::mod_api::{Hooks, Host, NetFromClientEvent};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod)
}

struct Mod;

/// The longest message accepted, in bytes (a chat line plus an item's components).
const MAX: usize = 32 * 1024;

impl Hooks for Mod {
    fn net_from_client(&self, host: &mut dyn Host, event: &mut NetFromClientEvent<'_>) {
        if event.channel != "smwyg:share" || event.data.len() > MAX {
            return;
        }
        let Ok(serde_json::Value::Object(payload)) = serde_json::from_slice::<serde_json::Value>(event.data) else {
            return;
        };
        let Some(body @ serde_json::Value::Array(_)) = payload.get("body") else {
            return;
        };
        // The server's own name for a player who has one; the window's own player (single player)
        // isn't on that list.
        let name = host
            .server_players()
            .into_iter()
            .find(|p| p.uuid == event.player)
            .map(|p| p.name)
            .or_else(|| payload.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .unwrap_or_default();
        let line = serde_json::json!({
            "translate": "chat.type.text",
            "with": [{ "text": name }, body],
        });
        if !host.run_command(&format!("tellraw @a {line}")) {
            eprintln!("[smwyg] tellraw failed");
        }
    }
}
