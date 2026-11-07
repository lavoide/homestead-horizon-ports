//! Crops Love Rain for Horizon: crops and sugar cane grow faster while it rains on them, as the
//! Fabric mod `crops_love_rain` 1.4.0 does with Homestead's config (`rainGrowthSpeed = 3`).
//!
//! The Java mod, at the start of a crop's random tick: if it's lit (raw brightness >= 9), not
//! fully grown and `level.isRainingAt(pos)`, then with chance 1/3 it adds one age stage.
//! Sugar cane the same, while the cane is less than 3 tall: +1 age, or at age 15 a new cane on
//! top.
//!
//! Horizon's random-tick hook gets a level host without block names or weather, so the work is
//! split: `random_tick` notes blocks with an `age` property; `level_tick_post` (same level, same
//! tick, full host) checks each one's name, the rain and the sky, and grows it.
use std::sync::Mutex;

use horizon_client::mod_api::{
    Flow, Hooks, Host, LevelTickPostEvent, LevelWriteHost, RandomTickEvent,
};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

/// Homestead's `rainGrowthSpeed`: one extra stage with chance 1 in this.
pub const RAIN_GROWTH_SPEED: u64 = 3;

/// The vanilla `CropBlock`s and their last age.
pub fn crop_max_age(block: &str) -> Option<u32> {
    match block {
        "minecraft:wheat" | "minecraft:carrots" | "minecraft:potatoes" => Some(7),
        "minecraft:beetroots" => Some(3),
        _ => None,
    }
}

/// What to do with a rained-on block: the new `age`, if it grows.
#[derive(Debug, PartialEq)]
pub enum Growth {
    Age(u32),
    /// Sugar cane at age 15: a new cane on top, this one back to age 0.
    NewCane,
}

/// The growth for `block` at `age` (with `height` canes in its column counting itself, for
/// sugar cane), before the 1-in-`RAIN_GROWTH_SPEED` roll.
pub fn growth(block: &str, age: u32, height: u32) -> Option<Growth> {
    if let Some(max) = crop_max_age(block) {
        return (age < max).then_some(Growth::Age(age + 1));
    }
    if block == "minecraft:sugar_cane" && height < 3 {
        return Some(if age >= 15 { Growth::NewCane } else { Growth::Age(age + 1) });
    }
    None
}

/// A small xorshift: the roll only needs to be fair, not vanilla's stream.
fn roll(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

struct State {
    queue: Vec<[i32; 3]>,
    seed: u64,
}

struct Mod {
    state: Mutex<State>,
}

impl Default for Mod {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
        Mod { state: Mutex::new(State { queue: Vec::new(), seed }) }
    }
}

impl Mod {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Hooks for Mod {
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        // Only blocks with an `age` property can be ours; the names are checked after the tick.
        if level.state_with_property(event.state, "age", "0").is_some() {
            let p = event.pos;
            self.lock().queue.push([p.x, p.y, p.z]);
        }
        Flow::Continue
    }

    fn level_tick_post(&self, host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        let pending = std::mem::take(&mut self.lock().queue);
        if pending.is_empty() || !host.on("cropsloverain") {
            return;
        }
        // `Level.isRaining`: rain level above 0.2. Nether and End report 0.
        let (rain, _) = host.weather();
        if rain <= 0.2 || event.dimension != "minecraft:overworld" {
            return;
        }
        for pos in pending {
            // Rained on: full sky light at the block (leaves and solid blocks above take it
            // down, as they stop rain). That also covers the Java mod's brightness >= 9 check.
            if host.light(pos).1 < 15 {
                continue;
            }
            let block = host.block(pos);
            let Some(age) = host.block_property(pos, "age").and_then(|a| a.parse::<u32>().ok())
            else {
                continue;
            };
            let height = cane_height(host, pos, &block);
            let Some(grow) = growth(&block, age, height) else { continue };
            if roll(&mut self.lock().seed) % RAIN_GROWTH_SPEED != 0 {
                continue;
            }
            match grow {
                Growth::Age(a) => {
                    host.set_block_property(pos, "age", &a.to_string());
                }
                Growth::NewCane => {
                    let above = [pos[0], pos[1] + 1, pos[2]];
                    if host.block(above) == "minecraft:air" {
                        let line = format!(
                            "execute in minecraft:overworld run setblock {} {} {} minecraft:sugar_cane",
                            above[0], above[1], above[2]
                        );
                        if host.run_command(&line) {
                            host.set_block_property(pos, "age", "0");
                        }
                    }
                }
            }
        }
    }
}

/// How many sugar canes stand in the column ending at `pos`, counting it (vanilla's check).
fn cane_height(host: &dyn Host, pos: [i32; 3], block: &str) -> u32 {
    if block != "minecraft:sugar_cane" {
        return 1;
    }
    let mut h = 1;
    while h < 3 && host.block([pos[0], pos[1] - h as i32, pos[2]]) == "minecraft:sugar_cane" {
        h += 1;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crops_grow_one_stage_until_grown() {
        assert_eq!(growth("minecraft:wheat", 3, 1), Some(Growth::Age(4)));
        assert_eq!(growth("minecraft:wheat", 7, 1), None);
        assert_eq!(growth("minecraft:beetroots", 2, 1), Some(Growth::Age(3)));
        assert_eq!(growth("minecraft:beetroots", 3, 1), None);
    }

    #[test]
    fn other_aged_blocks_are_not_crops() {
        assert_eq!(growth("minecraft:melon_stem", 3, 1), None);
        assert_eq!(growth("minecraft:nether_wart", 1, 1), None);
        assert_eq!(growth("minecraft:cactus", 1, 1), None);
    }

    #[test]
    fn sugar_cane_ages_then_grows_up_to_three_tall() {
        assert_eq!(growth("minecraft:sugar_cane", 4, 1), Some(Growth::Age(5)));
        assert_eq!(growth("minecraft:sugar_cane", 15, 2), Some(Growth::NewCane));
        assert_eq!(growth("minecraft:sugar_cane", 15, 3), None);
    }

    #[test]
    fn the_roll_hits_about_one_in_three() {
        let mut seed = 12345u64;
        let hits = (0..30_000).filter(|_| roll(&mut seed) % RAIN_GROWTH_SPEED == 0).count();
        assert!((9_000..11_000).contains(&hits), "{hits}");
    }
}
