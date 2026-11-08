// The add-ons' loot injections (Fabric LootTableEvents.MODIFY / REPLACE, Forge loot modifiers): a
// pool drawing the add-on's table, added to the vanilla table it extends. A missing add-on's table
// draws nothing.
fn fd_addon_loot(id: &str, table: &mut serde_json::Value) {
    let refs: &[&str] = match id {
        // OceansDelight.registerLootTable
        "minecraft:entities/squid" => &["oceansdelight:inject/entities/squid"],
        "minecraft:entities/guardian" => &["oceansdelight:inject/entities/guardian"],
        "minecraft:entities/elder_guardian" => &["oceansdelight:inject/entities/elder_guardian"],
        // EndsDelight.registerLootTable; Nether's Delight's machete on shulkers.
        "minecraft:entities/ender_dragon" => &["ends_delight:inject/entities/ender_dragon"],
        "minecraft:entities/enderman" => &["ends_delight:inject/entities/enderman"],
        "minecraft:entities/endermite" => &["ends_delight:inject/entities/endermite"],
        "minecraft:entities/shulker" => &["ends_delight:inject/entities/shulker", "nethersdelight:inject/entities/shulker"],
        // NDCommonSetup (the strider's slice) and Nether's Delight's loot modifiers (machetes).
        "minecraft:entities/strider" => &["nethersdelight:inject/entities/strider"],
        "minecraft:entities/pig" => &["nethersdelight:inject/entities/pig"],
        "minecraft:entities/chicken" => &["nethersdelight:inject/entities/chicken"],
        "minecraft:entities/rabbit" => &["nethersdelight:inject/entities/rabbit"],
        "minecraft:entities/cow" => &["nethersdelight:inject/entities/cow"],
        "minecraft:entities/donkey" => &["nethersdelight:inject/entities/donkey"],
        "minecraft:entities/horse" => &["nethersdelight:inject/entities/horse"],
        "minecraft:entities/llama" => &["nethersdelight:inject/entities/llama"],
        "minecraft:entities/mule" => &["nethersdelight:inject/entities/mule"],
        "minecraft:entities/spider" => &["nethersdelight:inject/entities/spider"],
        "minecraft:entities/cave_spider" => &["nethersdelight:inject/entities/cave_spider"],
        // Expanded Delight's LootTableUtil: crops in village house chests.
        "minecraft:chests/village/village_desert_house" => &["expandeddelight:inject/chests/village_desert_house"],
        "minecraft:chests/village/village_savanna_house" => &["expandeddelight:inject/chests/village_savanna_house"],
        "minecraft:chests/village/village_snowy_house" => &["expandeddelight:inject/chests/village_snowy_house"],
        // NDCommonSetup: the hoglin's table is Nether's Delight's (hoglin loin, leather unless a
        // hunting tool takes its hide).
        "minecraft:entities/hoglin" => {
            *table = serde_json::json!({"type": "minecraft:entity", "pools": [
                {"rolls": 1, "entries": [{"type": "minecraft:loot_table", "value": "nethersdelight:replace/entities/hoglin"}]}]});
            return;
        }
        // NDCommonSetup: propelpearls in every bartering pool, weight 20.
        "minecraft:gameplay/piglin_bartering" => {
            let Some(pools) = table.get_mut("pools").and_then(|p| p.as_array_mut()) else { return };
            for pool in pools {
                if let Some(entries) = pool.get_mut("entries").and_then(|e| e.as_array_mut()) {
                    entries.push(serde_json::json!({"type": "minecraft:loot_table", "weight": 20,
                        "value": "nethersdelight:inject/gameplay/piglin_bartering"}));
                }
            }
            return;
        }
        _ => return,
    };
    let Some(t) = table.as_object_mut() else { return };
    let Some(pools) = t.entry("pools").or_insert_with(|| serde_json::json!([])).as_array_mut() else { return };
    for r in refs {
        pools.push(serde_json::json!({"rolls": 1, "entries": [{"type": "minecraft:loot_table", "value": r}]}));
    }
}
