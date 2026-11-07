// Secure Crops for Horizon: `landing::load` from Horizon 3fb4c31, with farmland on the default
// landing rule. Injected as `crate::landing::securecrops`; the bundle's HEAD makes the original
// return this. Rebase it on a Horizon update (the compatibility pin makes the build say so).
use super::*;

pub fn load(root: &Path, properties: &MovementProperties<'_>) -> Result<BlockLandingRegistry> {
    let raw: BTreeMap<String, serde_json::Value> =
        serde_json::from_slice(&fs::read(root.join("landing-definitions.json"))?)?;
    let mut definitions = BTreeMap::new();
    for (name, d) in raw {
        let float = |key: &str| -> Result<f32> {
            Ok(f32::from_bits(u32::from_str_radix(
                d[key].as_str().ok_or("missing landing float")?,
                16,
            )?))
        };
        let fall = match d["fall"].as_str().ok_or("missing landing family")? {
            "Block" => FallDefinition::Default,
            "HayBlock" => FallDefinition::Hay,
            "HoneyBlock" => FallDefinition::Honey {
                volume: float("volume")?,
                pitch: float("pitch")?,
            },
            "SlimeBlock" => FallDefinition::Slime,
            "PowderSnowBlock" => FallDefinition::PowderSnow,
            "PointedDripstoneBlock" => FallDefinition::PointedDripstone,
            // Secure Crops: farmland lands as any block does, so nothing tramples it.
            "FarmlandBlock" => FallDefinition::Default,
            "TurtleEggBlock" => FallDefinition::TurtleEgg,
            _ => return Err("unsupported landing family".into()),
        };
        definitions.insert(
            name,
            LandingDefinition {
                fall,
                shelf_mushroom_bounce: d["bounce"].as_bool().ok_or("missing bounce flag")?,
            },
        );
    }
    Ok(BlockLandingRegistry::new(properties, &definitions)?)
}
