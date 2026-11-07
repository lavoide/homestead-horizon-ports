//! Obsidian Equipment Reworked's chest loot (ModLootTableModifiers, LootTableEvents.MODIFY):
//! a pool of one roll with a chance of the obsidian horse armour or the obsidian upgrade
//! smithing template, added to vanilla's chests.
use horizon_client::mod_api::serde_json::{Value, json};
use horizon_client::mod_api::{Hooks, LootTableLoadEvent};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod)
}

const HORSE_ARMOR: &str = "obsidianequipmentrework:obsidian_horse_armor";
const TEMPLATE: &str = "obsidianequipmentrework:obsidian_upgrade_smithing_template";

/// The tables and chances: OBSIDIAN_HORSE_ARMOR_LOOT_TABLES (0.18) and
/// OBSIDIAN_UPGRADE_TEMPLATE_LOOT_TABLES.
fn extras(id: &str) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    if matches!(
        id,
        "minecraft:chests/nether_bridge" | "minecraft:chests/bastion_treasure" | "minecraft:chests/ruined_portal"
    ) {
        out.push((HORSE_ARMOR, 0.18));
    }
    let template = match id {
        "minecraft:chests/bastion_treasure" => 0.2,
        "minecraft:chests/bastion_other" | "minecraft:chests/bastion_bridge" => 0.13,
        "minecraft:chests/nether_bridge" | "minecraft:chests/ancient_city" => 0.15,
        "minecraft:chests/abandoned_mineshaft" => 0.11,
        "minecraft:chests/stronghold_corridor" => 0.13,
        "minecraft:chests/ruined_portal" => 0.1,
        _ => 0.0,
    };
    if template > 0.0 {
        out.push((TEMPLATE, template));
    }
    out
}

/// LootPool.builder().rolls(1).conditionally(RandomChance(chance)).with(item): one roll whose
/// only entry holds with that chance.
fn pool(item: &str, chance: f64) -> Value {
    json!({
        "rolls": 1,
        "entries": [{
            "type": "minecraft:item",
            "name": item,
            "condition": { "type": "minecraft:random_chance", "chance": chance }
        }]
    })
}

struct Mod;

impl Hooks for Mod {
    fn loot_table_load(&self, event: &mut LootTableLoadEvent<'_>) {
        let extras = extras(event.id);
        if extras.is_empty() {
            return;
        }
        let Some(table) = event.table.as_object_mut() else { return };
        let pools = table.entry("pools").or_insert_with(|| json!([]));
        if let Some(pools) = pools.as_array_mut() {
            for (item, chance) in extras {
                pools.push(pool(item, chance));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nether_fortress_gets_both() {
        let got = extras("minecraft:chests/nether_bridge");
        assert_eq!(got, vec![(HORSE_ARMOR, 0.18), (TEMPLATE, 0.15)]);
        assert!(extras("minecraft:chests/simple_dungeon").is_empty());
    }
}
