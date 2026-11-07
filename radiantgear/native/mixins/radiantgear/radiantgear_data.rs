// Written by tools/trinkets_port.py: LambDynamicLights' item lights in the pack's jars
// (assets/<ns>/dynamiclights/item/*.json), as (item, luminance, water sensitive).
pub(crate) const ITEM_LIGHTS: &[(&str, u8, bool)] = &[
    ("trinketlantern:carry_lantern", 15, true),
    ("trinketlantern:carry_lantern_waterproof", 15, false),
    ("trinketlantern:carry_soul_lantern", 10, true),
    ("trinketlantern:carry_soul_lantern_waterproof", 10, false),
];
