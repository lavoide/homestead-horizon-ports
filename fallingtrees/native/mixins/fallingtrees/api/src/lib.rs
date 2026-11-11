//! Falling Trees for Horizon: chopping a tree's log with an axe takes the whole tree down, as
//! Falling Trees 0.13.2 (PandaMods) does with Homestead's config (generic trees, axes only,
//! 256 logs at most, leaves within 10, persistent leaves left alone, sneak to chop one log).
//!
//! The tree, as `GenericTree.gatherTreeData` finds it: the logs reachable from the chopped one
//! through the 3x3 squares on its level and the level above (never down), then for each log the
//! leaves whose `distance` matches their path from it (so another tree's leaves don't count), and
//! the vines, cocoa and bee nests touching any of them. No leaves: not a tree.
//!
//! `destroy_block` (a player hook: reads only) finds the tree and lets the chopped log break as
//! vanilla does; the next server tick drops every other block's loot, mined with the held tool,
//! at the chopped log (`loot spawn ... mine ... mainhand`), clears the blocks (attachments,
//! leaves, then logs, so nothing pops twice), and wears the axe one point per extra log.
//!
//! Sneaking: the host's comes from the client's `movement_input` (same process), a LAN guest's
//! from `player_toggle_sneak`.
//!
//! Not ported: the falling animation and its sounds, the 4 s before the drops land, slower
//! mining for bigger trees (no mining-speed hook), extra hunger (no exhaustion command),
//! Unbreaking on the extra wear, and the cactus, bamboo and chorus "trees" (vanilla already
//! breaks those upwards). Log and leaf tags are matched by name (no block tag API).
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_client::mod_api::{
    DestroyBlockEvent, Flow, Hooks, Host, MovementInputEvent, PlayerToggleSneakEvent,
    ServerTickEvent, ServerTickPhase,
};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

pub const MAX_LOGS: usize = 256;
pub const MAX_LEAVES_RADIUS: u32 = 10;

/// `#minecraft:logs` (and `#c:logs`) by name.
pub fn is_log(block: &str) -> bool {
    block != "minecraft:mushroom_stem"
        && ["_log", "_wood", "_stem", "_hyphae"].iter().any(|s| block.ends_with(s))
}

/// `#minecraft:leaves` by name.
pub fn is_leaves(block: &str) -> bool {
    block.ends_with("_leaves")
}

/// `#minecraft:axes` by name.
pub fn is_axe(item: &str) -> bool {
    item.ends_with("_axe") && !item.ends_with("_pickaxe")
}

/// The durability of the axes, for the extra wear.
pub fn axe_durability(item: &str) -> Option<u32> {
    Some(match item {
        "minecraft:wooden_axe" => 59,
        "minecraft:golden_axe" => 32,
        "minecraft:stone_axe" => 131,
        "minecraft:copper_axe" => 190,
        "minecraft:iron_axe" => 250,
        "minecraft:diamond_axe" => 1561,
        "minecraft:netherite_axe" => 2031,
        // The other axes of Homestead (no host call gives an item's max damage):
        // obsidianequipmentrework's, and tokimistoolshed's lumber axes (ModToolMaterials).
        "obsidianequipmentrework:obsidian_axe" => 1800,
        "tokimistoolshed:wooden_lumber_axe" => 144,
        "tokimistoolshed:stone_lumber_axe" => 324,
        "tokimistoolshed:iron_lumber_axe" => 621,
        "tokimistoolshed:golden_lumber_axe" => 81,
        "tokimistoolshed:diamond_lumber_axe" => 3906,
        "tokimistoolshed:netherite_lumber_axe" => 5076,
        "tokimistoolshed:obsidian_lumber_axe" => 4000,
        _ => return None,
    })
}

/// What the finder needs to know about a block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub name: String,
    /// The leaves' `distance`, if the block has one.
    pub distance: Option<u32>,
    pub persistent: bool,
}

/// A tree to take down, apart from the chopped log.
#[derive(Debug, Default, PartialEq)]
pub struct Tree {
    pub logs: Vec<[i32; 3]>,
    pub leaves: Vec<[i32; 3]>,
    /// Vines, cocoa and bee nests.
    pub attached: Vec<[i32; 3]>,
}

const SIDES: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

fn add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// The tree whose log at `start` is chopped, or None if it isn't one (no leaves, too big).
pub fn find_tree(start: [i32; 3], read: &dyn Fn([i32; 3]) -> Block) -> Option<Tree> {
    // gatherLogs: breadth-first through dx, dz in -1..=1, dy in 0..=1.
    let mut logs = Vec::new();
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(pos) = queue.pop_front() {
        if !is_log(&read(pos).name) {
            continue;
        }
        logs.push(pos);
        if logs.len() > MAX_LOGS {
            return None;
        }
        for dy in 0..=1 {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let next = add(pos, [dx, dy, dz]);
                    if seen.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }
    }
    if logs.is_empty() {
        return None;
    }
    // gatherLeavesAroundLog, for each log; a leaf counts where its distance is its depth.
    let mut leaves = HashSet::new();
    for &log in &logs {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<([i32; 3], u32)> = SIDES.iter().map(|&d| (add(log, d), 1)).collect();
        while let Some((pos, depth)) = queue.pop_front() {
            if depth > MAX_LEAVES_RADIUS || visited.contains(&pos) {
                continue;
            }
            let b = read(pos);
            if b.distance.unwrap_or(0) != depth {
                continue;
            }
            visited.insert(pos);
            if is_leaves(&b.name) && !b.persistent {
                leaves.insert(pos);
                for d in SIDES {
                    let next = add(pos, d);
                    if !visited.contains(&next) {
                        queue.push_back((next, depth + 1));
                    }
                }
            }
        }
    }
    if leaves.is_empty() {
        return None;
    }
    // gatherAdjacentBlocks: vines (and the vines hanging below them), cocoa, bee nests.
    let mut attached = HashSet::new();
    for &pos in logs.iter().chain(leaves.iter()) {
        for d in SIDES {
            let mut next = add(pos, d);
            match read(next).name.as_str() {
                "minecraft:vine" => {
                    while read(next).name == "minecraft:vine" && attached.insert(next) {
                        next = add(next, [0, -1, 0]);
                    }
                }
                "minecraft:cocoa" | "minecraft:bee_nest" => {
                    attached.insert(next);
                }
                _ => {}
            }
        }
    }
    let mut leaves: Vec<_> = leaves.into_iter().collect();
    let mut attached: Vec<_> = attached.into_iter().collect();
    leaves.sort();
    attached.sort();
    logs.retain(|&p| p != start);
    Some(Tree { logs, leaves, attached })
}

/// One felled tree, waiting for the server tick.
#[derive(Debug)]
pub struct Felling {
    pub dimension: String,
    pub player: String,
    pub origin: [i32; 3],
    pub tree: Tree,
    /// The axe to wear, if survival and a known one.
    pub axe: Option<String>,
}

impl Felling {
    fn loot_line(&self, [bx, by, bz]: [i32; 3]) -> String {
        let [ox, oy, oz] = self.origin;
        let (x, y, z) = (f64::from(ox) + 0.5, f64::from(oy) + 0.125, f64::from(oz) + 0.5);
        format!(
            "execute in {} as {} run loot spawn {x:.1} {y:.3} {z:.1} mine {bx} {by} {bz} mainhand",
            self.dimension, self.player
        )
    }
    fn clear_line(&self, [bx, by, bz]: [i32; 3]) -> String {
        format!("execute in {} run setblock {bx} {by} {bz} minecraft:air", self.dimension)
    }
    /// The axe's wear for `logs` felled logs, if it's a known axe.
    fn wear_line(&self, logs: usize) -> Option<String> {
        let max = self.axe.as_deref().and_then(axe_durability)?;
        if logs == 0 {
            return None;
        }
        Some(format!(
            "execute as {} run item modify entity @s weapon.mainhand {{\"type\":\"minecraft:set_damage\",\"damage\":{:.6},\"add\":true}}",
            self.player,
            // `SetItemDamageFunction` floors a float: half a point keeps it from landing one short.
            -(logs as f64 + 0.5) / f64::from(max)
        ))
    }

    /// Fells the tree with `run`: each block's drops spawn at the stump, then the block is
    /// cleared. A block whose loot command fails is left standing, so nothing is removed without
    /// its drops; the axe wears for the logs actually felled. Returns how many blocks were felled.
    pub fn run(&self, run: &mut dyn FnMut(&str) -> bool) -> usize {
        let mut felled = 0;
        let mut logs = 0;
        let blocks = self.tree.attached.iter().chain(&self.tree.leaves).map(|b| (b, false));
        for (&b, is_log) in blocks.chain(self.tree.logs.iter().map(|b| (b, true))) {
            let loot = self.loot_line(b);
            if !run(&loot) {
                eprintln!("[fallingtrees] command failed, block left standing: {loot}");
                continue;
            }
            let clear = self.clear_line(b);
            if !run(&clear) {
                eprintln!("[fallingtrees] command failed: {clear}");
                continue;
            }
            felled += 1;
            logs += usize::from(is_log);
        }
        if let Some(line) = self.wear_line(logs)
            && !run(&line)
        {
            eprintln!("[fallingtrees] command failed: {line}");
        }
        felled
    }
}

#[derive(Default)]
struct Mod {
    queue: Mutex<Vec<Felling>>,
    /// The local player's sneak, from the client side of this process.
    host_sneaking: AtomicBool,
    /// LAN guests' sneak.
    guests_sneaking: Mutex<HashMap<u128, bool>>,
}

fn read_block(host: &dyn Host, pos: [i32; 3]) -> Block {
    Block {
        name: host.block(pos),
        distance: host.block_property(pos, "distance").and_then(|d| d.parse().ok()),
        persistent: host.block_property(pos, "persistent").as_deref() == Some("true"),
    }
}

impl Hooks for Mod {
    fn movement_input(&self, _host: &dyn Host, event: &mut MovementInputEvent) {
        self.host_sneaking.store(event.sneak, Ordering::Relaxed);
    }

    fn player_toggle_sneak(&self, _host: &mut dyn Host, event: &mut PlayerToggleSneakEvent<'_>) {
        let mut guests = self.guests_sneaking.lock().unwrap_or_else(|e| e.into_inner());
        guests.insert(event.player, event.sneaking);
    }

    fn destroy_block(&self, host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        if !host.on("fallingtrees") {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else { return Flow::Continue };
        if player.game_mode > 1 {
            return Flow::Continue;
        }
        let sneaking = {
            let guests = self.guests_sneaking.lock().unwrap_or_else(|e| e.into_inner());
            guests.get(&player.uuid).copied().unwrap_or_else(|| self.host_sneaking.load(Ordering::Relaxed))
        };
        if sneaking {
            return Flow::Continue;
        }
        let Some((item, _, _)) = host.held() else { return Flow::Continue };
        if !is_axe(&item) {
            return Flow::Continue;
        }
        let read = |p: [i32; 3]| read_block(host, p);
        let Some(tree) = find_tree(event.pos, &read) else { return Flow::Continue };
        let axe = (player.game_mode == 0).then_some(item);
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.push(Felling { dimension: player.dimension, player: player.name, origin: event.pos, tree, axe });
        Flow::Continue
    }

    fn server_tick(&self, host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let pending = std::mem::take(&mut *self.queue.lock().unwrap_or_else(|e| e.into_inner()));
        for felling in pending {
            felling.run(&mut |line| host.run_command(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(blocks: &[([i32; 3], &str, Option<u32>)]) -> HashMap<[i32; 3], Block> {
        blocks
            .iter()
            .map(|&(p, n, d)| (p, Block { name: n.into(), distance: d, persistent: false }))
            .collect()
    }

    fn reader(w: &HashMap<[i32; 3], Block>) -> impl Fn([i32; 3]) -> Block + '_ {
        |p| w.get(&p).cloned().unwrap_or(Block { name: "minecraft:air".into(), ..Block::default() })
    }

    #[test]
    fn names() {
        assert!(is_log("minecraft:oak_log") && is_log("minecraft:stripped_birch_wood"));
        assert!(is_log("minecraft:crimson_stem") && !is_log("minecraft:mushroom_stem"));
        assert!(is_leaves("minecraft:azalea_leaves") && !is_leaves("minecraft:oak_log"));
        assert!(is_axe("minecraft:iron_axe") && !is_axe("minecraft:iron_pickaxe"));
    }

    #[test]
    fn a_small_tree_falls_whole() {
        let w = world(&[
            ([0, 0, 0], "minecraft:oak_log", None),
            ([0, 1, 0], "minecraft:oak_log", None),
            ([0, 2, 0], "minecraft:oak_log", None),
            ([1, 2, 0], "minecraft:oak_leaves", Some(1)),
            ([2, 2, 0], "minecraft:oak_leaves", Some(2)),
            ([0, 3, 0], "minecraft:oak_leaves", Some(1)),
            ([1, 1, 0], "minecraft:cocoa", None),
            ([2, 1, 0], "minecraft:vine", None),
            ([2, 0, 0], "minecraft:vine", None),
        ]);
        let t = find_tree([0, 0, 0], &reader(&w)).unwrap();
        assert_eq!(t.logs, vec![[0, 1, 0], [0, 2, 0]]);
        assert_eq!(t.leaves, vec![[0, 3, 0], [1, 2, 0], [2, 2, 0]]);
        assert_eq!(t.attached, vec![[1, 1, 0], [2, 0, 0], [2, 1, 0]]);
    }

    #[test]
    fn logs_below_and_other_trees_leaves_stay() {
        let w = world(&[
            ([0, -1, 0], "minecraft:oak_log", None),
            ([0, 0, 0], "minecraft:oak_log", None),
            ([0, 1, 0], "minecraft:oak_leaves", Some(1)),
            // Distance 1 from another log, two steps from ours: not ours.
            ([1, 1, 0], "minecraft:oak_leaves", Some(1)),
        ]);
        let t = find_tree([0, 0, 0], &reader(&w)).unwrap();
        assert!(t.logs.is_empty());
        assert_eq!(t.leaves, vec![[0, 1, 0]]);
    }

    #[test]
    fn no_leaves_or_persistent_leaves_is_no_tree() {
        let mut w = world(&[
            ([0, 0, 0], "minecraft:oak_log", None),
            ([0, 1, 0], "minecraft:oak_log", None),
        ]);
        assert_eq!(find_tree([0, 0, 0], &reader(&w)), None);
        w.insert([0, 2, 0], Block { name: "minecraft:oak_leaves".into(), distance: Some(1), persistent: true });
        assert_eq!(find_tree([0, 0, 0], &reader(&w)), None);
    }

    #[test]
    fn too_big_is_no_tree() {
        let mut w = world(&[([0, 300, 0], "minecraft:oak_leaves", Some(1))]);
        for y in 0..300 {
            w.insert([0, y, 0], Block { name: "minecraft:oak_log".into(), ..Block::default() });
        }
        assert_eq!(find_tree([0, 0, 0], &reader(&w)), None);
    }

    fn felling() -> Felling {
        Felling {
            dimension: "minecraft:overworld".into(),
            player: "Steve".into(),
            origin: [-3, 64, 2],
            tree: Tree { logs: vec![[-3, 65, 2]], leaves: vec![[-3, 66, 2]], attached: vec![] },
            axe: Some("minecraft:iron_axe".into()),
        }
    }

    #[test]
    fn each_block_drops_at_the_stump_then_clears_then_the_axe_wears() {
        let mut c = Vec::new();
        let felled = felling().run(&mut |l| {
            c.push(l.to_string());
            true
        });
        assert_eq!(felled, 2);
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], "execute in minecraft:overworld as Steve run loot spawn -2.5 64.125 2.5 mine -3 66 2 mainhand");
        assert_eq!(c[1], "execute in minecraft:overworld run setblock -3 66 2 minecraft:air");
        assert_eq!(c[3], "execute in minecraft:overworld run setblock -3 65 2 minecraft:air");
        assert!(c[4].ends_with("{\"type\":\"minecraft:set_damage\",\"damage\":-0.006000,\"add\":true}"), "{}", c[4]);
    }

    #[test]
    fn a_block_whose_loot_fails_stays_and_doesnt_wear_the_axe() {
        let mut c = Vec::new();
        // The log's loot fails: it isn't cleared, and with no log felled the axe doesn't wear.
        let felled = felling().run(&mut |l| {
            c.push(l.to_string());
            !(l.contains("loot spawn") && l.contains("mine -3 65 2"))
        });
        assert_eq!(felled, 1);
        assert!(!c.iter().any(|l| l.contains("setblock -3 65 2")));
        assert!(!c.iter().any(|l| l.contains("set_damage")));
    }
}
