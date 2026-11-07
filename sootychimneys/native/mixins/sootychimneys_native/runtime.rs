// Sooty Chimneys 1.3.4 (mortuusars): ChimneyBlock.use. The chimneys are the content mod
// mods/sootychimneys; their ticking is native/sootychimneys/api.
//
// Injected as `crate::game::sootychimneys`.
use super::*;

const PREFIX: &str = "sootychimneys:";

fn name_at(game: &Game<'_>, p: [i32; 3]) -> std::sync::Arc<str> {
    game.res.facts().get(game.world.block(p[0], p[1], p[2])).name.clone()
}

fn is_chimney(name: &str) -> bool {
    name.strip_prefix(PREFIX).is_some_and(|n| n.ends_with("_chimney"))
}

/// The state at `at` with `name` for its block and its own `lit`/`blocked`/`stacked`, with
/// `changes` applied.
fn restate(game: &Game<'_>, at: [i32; 3], name: &str, changes: &[(&str, &str)]) -> Option<horizon_world::registry::BlockStateId> {
    let reg = game.pack.assets.registry;
    let old = reg.state(game.world.block(at[0], at[1], at[2]))?.clone();
    let mut st = reg.state(reg.default_state(name)?)?.clone();
    for key in ["blocked", "lit", "stacked"] {
        if let Some(v) = old.properties.get(key) {
            st.properties.insert(key.into(), v.clone());
        }
    }
    for (k, v) in changes {
        st.properties.insert((*k).into(), (*v).into());
    }
    reg.resolve(&st).ok()
}

/// SootScrapingRecipe results (data/sootychimneys/recipe/soot_scraping): black dye at a chance.
fn scraping_chance(dirty: &str) -> f32 {
    match dirty.strip_prefix("sootychimneys:dirty_").unwrap_or("") {
        "iron_chimney" | "copper_chimney" => 0.5,
        _ => 0.75,
    }
}

fn is_axe(game: &Game<'_>, item: &str) -> bool {
    game.res.items().item_tags.get("minecraft:axes").is_some_and(|t| t.contains(item))
}

/// ChimneyBlock.use: true when the click was the chimney's.
pub(crate) fn use_chimney(game: &mut Game<'_>, at: [i32; 3], side: [i32; 3]) -> bool {
    let name = name_at(game, at);
    if !is_chimney(&name) {
        return false;
    }
    let main = game.hand == horizon_client::first_person::Hand::Main;
    let held = game.held().cloned();
    let center = at.map(|c| f64::from(c) + 0.5);
    if main && held.is_none() {
        // The smoke blocked or let through, with the dispenser's or the iron trapdoor's
        // sound and a line over the hotbar.
        let reg = game.pack.assets.registry;
        let blocked = reg
            .state(game.world.block(at[0], at[1], at[2]))
            .and_then(|s| s.properties.get("blocked").cloned())
            .is_some_and(|v| v == "true");
        let value = if blocked { "false" } else { "true" };
        let Some(state) = restate(game, at, &name, &[("blocked", value)]) else { return false };
        if game.level_set(at, state).is_err() {
            return false;
        }
        let pitch = 0.85 + game.world.random.next_float() * 0.05;
        let sound = if blocked { "minecraft:block.iron_trapdoor.open" } else { "minecraft:block.iron_trapdoor.close" };
        game.audio.play(sound, Some(center), 0.8, pitch);
        if let Some(player) = game.native_acting_player() {
            let key = if blocked { "message.sootychimneys.open" } else { "message.sootychimneys.blocked" };
            game.native_action_bar(player.uuid, horizon_client::text::Text::translate(key, &[]));
        }
        return true;
    }
    // Scraping a dirty chimney clean with a soot scraper (#minecraft:axes).
    let Some(tool) = held.filter(|s| is_axe(game, &s.item)) else { return false };
    let Some(clean) = name.strip_prefix("sootychimneys:dirty_") else { return false };
    let clean = format!("{PREFIX}{clean}");
    let Some(state) = restate(game, at, &clean, &[]) else { return false };
    if game.level_set(at, state).is_err() {
        return false;
    }
    if game.world.random.next_float() < scraping_chance(&name) {
        let n = side.map(f64::from);
        let spot = [center[0] + n[0] * 0.65, f64::from(at[1]) + 0.6 + n[1] * 0.65, center[2] + n[2] * 0.65];
        game.spawn_item(ItemStack::new("minecraft:black_dye", 1), spot, [0.0, 0.1, 0.0], 10);
    }
    game.damage_held(1);
    game.audio.play("minecraft:item.axe.scrape", Some(center), 1.0, 1.0);
    game.trigger_block("minecraft:item_used_on_block", at, &tool);
    true
}
