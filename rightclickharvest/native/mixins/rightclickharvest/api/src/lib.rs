//! Right Click Harvest for Horizon: right-click a fully grown crop to harvest it and replant it,
//! as the Fabric mod `rightclickharvest` (Homestead 2.0) does.
//!
//! The click (a player hook) only checks the crop and claims the click. Player hooks can't run
//! commands, so the harvest itself waits for the next server tick: vanilla's
//! `loot spawn ... mine ... mainhand` drops what breaking the crop with the held item would
//! (Fortune included), then `setblock` puts the crop back at age 0.
//!
//! Difference from the Java mod: the Java mod takes one seed out of the drops for the replant;
//! this port leaves the drops as vanilla rolls them (one seed more per harvest).
use std::sync::Mutex;

use horizon_client::mod_api::{Flow, Hooks, Host, RightClickBlockEvent, ServerTickEvent};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

/// A crop this mod harvests: its id, the age it's grown at, and properties to keep.
struct Crop {
    id: &'static str,
    max_age: u32,
    keep: &'static [&'static str],
}

const CROPS: &[Crop] = &[
    Crop { id: "minecraft:wheat", max_age: 7, keep: &[] },
    Crop { id: "minecraft:carrots", max_age: 7, keep: &[] },
    Crop { id: "minecraft:potatoes", max_age: 7, keep: &[] },
    Crop { id: "minecraft:beetroots", max_age: 3, keep: &[] },
    Crop { id: "minecraft:nether_wart", max_age: 3, keep: &[] },
    Crop { id: "minecraft:cocoa", max_age: 2, keep: &["facing"] },
];

/// One queued harvest.
#[derive(Debug, Clone, PartialEq)]
pub struct Harvest {
    pub dimension: String,
    pub player: String,
    pub pos: [i32; 3],
    /// The state to put back, as `/setblock` takes it.
    pub replant: String,
}

impl Harvest {
    /// The commands that do the harvest, in order.
    pub fn commands(&self) -> [String; 2] {
        let [x, y, z] = self.pos;
        [
            format!(
                "execute in {} as {} run loot spawn {:.1} {:.1} {:.1} mine {x} {y} {z} mainhand",
                self.dimension,
                self.player,
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
                f64::from(z) + 0.5
            ),
            format!("execute in {} run setblock {x} {y} {z} {}", self.dimension, self.replant),
        ]
    }
}

/// Runs a harvest's commands in order and stops at the first one that fails, so a crop whose
/// loot didn't drop is never reset. True when every command ran.
pub fn run_harvest(harvest: &Harvest, run: &mut dyn FnMut(&str) -> bool) -> bool {
    for line in harvest.commands() {
        if !run(&line) {
            eprintln!("[rightclickharvest] command failed, harvest stopped: {line}");
            return false;
        }
    }
    true
}

/// The replanted state for `block` if it's a supported crop at full age.
/// `prop` reads a property of the clicked block.
pub fn replant_state(block: &str, prop: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let crop = CROPS.iter().find(|c| c.id == block)?;
    let age: u32 = prop("age")?.parse().ok()?;
    if age < crop.max_age {
        return None;
    }
    let mut props = vec!["age=0".to_string()];
    for key in crop.keep {
        props.push(format!("{key}={}", prop(key)?));
    }
    Some(format!("{}[{}]", crop.id, props.join(",")))
}

#[derive(Default)]
struct Mod {
    queue: Mutex<Vec<Harvest>>,
}

impl Hooks for Mod {
    fn right_click_block(&self, host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        if !host.on("rightclickharvest") {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else {
            return Flow::Continue;
        };
        // Spectators can't harvest; adventure mode can't break blocks.
        if player.game_mode >= 2 {
            return Flow::Continue;
        }
        let pos = event.pos;
        let block = host.block(pos);
        let read = |key: &str| host.block_property(pos, key);
        let Some(replant) = replant_state(&block, &read) else {
            return Flow::Continue;
        };
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        // The hook runs once per hand: queue each crop once.
        if !queue.iter().any(|h| h.pos == pos && h.dimension == player.dimension) {
            queue.push(Harvest { dimension: player.dimension, player: player.name, pos, replant });
        }
        Flow::Cancel
    }

    fn server_tick(&self, host: &mut dyn Host, _event: &mut ServerTickEvent<'_>) {
        let pending: Vec<Harvest> = {
            let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *queue)
        };
        for harvest in pending {
            run_harvest(&harvest, &mut |line| host.run_command(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(list: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| list.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn grown_wheat_replants_at_age_zero() {
        let p = props(&[("age", "7")]);
        assert_eq!(replant_state("minecraft:wheat", &p).as_deref(), Some("minecraft:wheat[age=0]"));
    }

    #[test]
    fn young_crops_and_other_blocks_are_left_alone() {
        assert_eq!(replant_state("minecraft:wheat", &props(&[("age", "6")])), None);
        assert_eq!(replant_state("minecraft:stone", &props(&[("age", "7")])), None);
        assert_eq!(replant_state("minecraft:beetroots", &props(&[("age", "2")])), None);
    }

    #[test]
    fn cocoa_keeps_its_facing() {
        let p = props(&[("age", "2"), ("facing", "west")]);
        assert_eq!(
            replant_state("minecraft:cocoa", &p).as_deref(),
            Some("minecraft:cocoa[age=0,facing=west]")
        );
    }

    fn carrots() -> Harvest {
        Harvest {
            dimension: "minecraft:overworld".into(),
            player: "Steve".into(),
            pos: [-3, 64, 12],
            replant: "minecraft:carrots[age=0]".into(),
        }
    }

    #[test]
    fn failed_loot_leaves_the_crop_alone() {
        let mut ran = Vec::new();
        let done = run_harvest(&carrots(), &mut |line| {
            ran.push(line.to_string());
            false
        });
        assert!(!done);
        assert_eq!(ran.len(), 1, "setblock must not run after the loot command fails");
        assert!(ran[0].contains("loot spawn"));
    }

    #[test]
    fn successful_harvest_loots_then_replants() {
        let mut ran = Vec::new();
        let done = run_harvest(&carrots(), &mut |line| {
            ran.push(line.to_string());
            true
        });
        assert!(done);
        assert_eq!(ran, carrots().commands().to_vec());
    }

    #[test]
    fn commands_loot_then_replant_in_the_players_dimension() {
        let h = Harvest {
            dimension: "minecraft:overworld".into(),
            player: "Steve".into(),
            pos: [-3, 64, 12],
            replant: "minecraft:carrots[age=0]".into(),
        };
        assert_eq!(
            h.commands(),
            [
                "execute in minecraft:overworld as Steve run loot spawn -2.5 64.5 12.5 mine -3 64 12 mainhand".to_string(),
                "execute in minecraft:overworld run setblock -3 64 12 minecraft:carrots[age=0]".to_string(),
            ]
        );
    }
}
