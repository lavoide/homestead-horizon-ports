// The add-ons' loot injections (Fabric LootTableEvents.MODIFY): a pool drawing the add-on's
// inject table, added to the vanilla table it extends. A missing add-on's table draws nothing.
fn fd_addon_loot(id: &str, table: &mut serde_json::Value) {
    let refs: &[&str] = match id {
        // OceansDelight.registerLootTable
        "minecraft:entities/squid" => &["oceansdelight:inject/entities/squid"],
        "minecraft:entities/guardian" => &["oceansdelight:inject/entities/guardian"],
        "minecraft:entities/elder_guardian" => &["oceansdelight:inject/entities/elder_guardian"],
        // EndsDelight.registerLootTable
        "minecraft:entities/ender_dragon" => &["ends_delight:inject/entities/ender_dragon"],
        "minecraft:entities/enderman" => &["ends_delight:inject/entities/enderman"],
        "minecraft:entities/endermite" => &["ends_delight:inject/entities/endermite"],
        "minecraft:entities/shulker" => &["ends_delight:inject/entities/shulker"],
        _ => return,
    };
    let Some(t) = table.as_object_mut() else { return };
    let Some(pools) = t.entry("pools").or_insert_with(|| serde_json::json!([])).as_array_mut() else { return };
    for r in refs {
        pools.push(serde_json::json!({"rolls": 1, "entries": [{"type": "minecraft:loot_table", "value": r}]}));
    }
}
