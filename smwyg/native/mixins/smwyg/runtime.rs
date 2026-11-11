// Show Me What You Got 1.1.1 (Apace, MIT), the window's half.
use super::*;
use std::sync::Mutex;

/// The shared item while the chat holds its name: (the name as typed, its hoverable component).
static SHARE: Mutex<Option<(String, serde_json::Value)>> = Mutex::new(None);

pub(crate) fn forget() {
    *SHARE.lock().unwrap() = None;
}

/// Ctrl + T over a filled slot of an open inventory, with nothing on the cursor: close the
/// inventory and open the chat with the item's `[name]`.
pub(crate) fn share_key(game: &mut Game<'_>, key: KeyCode) -> bool {
    if key != KeyCode::KeyT || !game.modifiers.control_key() || game.screen != Screen::Inventory || game.carried.is_some() {
        return false;
    }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else {
        return false;
    };
    let batch = HudBatch::new(size);
    let Some(slot) = game.slot_at(&batch, game.cursor.0, game.cursor.1) else {
        return false;
    };
    let Some(stack) = game.slot_view(slot).filter(|s| s.count > 0 && &*s.item != "minecraft:air") else {
        return false;
    };
    let text = game.stack_display_name(&stack);
    let name = text.resolved().plain();
    let mut component = text.component();
    // getDisplayName's hover with the whole stack (Horizon's own leaves the components out), so the
    // tooltip shows the name, enchantments and the rest.
    let item = horizon_client_core::scoreboard::tag_to_json(&horizon_items::item_nbt::item_to_nbt(&stack));
    if let (serde_json::Value::Object(mut event), serde_json::Value::Object(c)) = (item, &mut component) {
        event.insert("action".into(), "show_item".into());
        c.insert("hover_event".into(), serde_json::Value::Object(event));
    }
    game.close_container();
    game.screen = Screen::Playing;
    game.open_chat(&name, false);
    *SHARE.lock().unwrap() = Some((name, component));
    true
}

/// The chat line is being sent: if it still holds the shared name, send it to the server as
/// `smwyg:share` (the text before, the item, the text after) in its place.
pub(crate) fn send(game: &mut Game<'_>, message: &str) -> bool {
    let Some((name, component)) = SHARE.lock().unwrap().take() else {
        return false;
    };
    if message.starts_with('/') {
        return false;
    }
    let Some(at) = message.find(&name) else {
        return false;
    };
    let before = &message[..at];
    let after = &message[at + name.len()..];
    let own = game.player_display_name().resolved().plain();
    let payload = serde_json::json!({
        "name": own,
        "body": ["", before, component, after],
    });
    game.ui.chat.add_recent(message);
    if game.native_net_send_to_server("smwyg:share", payload.to_string().as_bytes()).is_err() {
        // No server to show it (a vanilla server): send the plain line.
        return false;
    }
    true
}
