//! Accelerated Decay for Horizon: leaves cut off from their logs decay within about a second
//! instead of waiting for random ticks, as the Fabric mod `accelerateddecay` 3.0.1 does.
//!
//! When a leaf block's state becomes `distance=7` and not persistent (`block_changed`, which
//! only has state ids, so the properties are tested through `state_with_property`), it is queued
//! with a short random delay; `level_tick_post` (full host) re-checks it and breaks it with
//! drops (`setblock ... air destroy`), as vanilla's decay drops its loot.
use std::sync::Mutex;

use horizon_client::mod_api::{BlockChangedEvent, Hooks, Host, LevelHost, LevelTickPostEvent};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

/// Most ticks a cut-off leaf waits (spread, so a tree falls apart over a moment, not at once).
pub const MAX_DELAY: u32 = 20;

/// Whether a leaf block with these property answers will decay.
pub fn decays(distance: Option<&str>, persistent: Option<&str>) -> bool {
    distance == Some("7") && persistent == Some("false")
}

pub fn break_command(dimension: &str, p: [i32; 3]) -> String {
    format!("execute in {dimension} run setblock {} {} {} minecraft:air destroy", p[0], p[1], p[2])
}

struct Pending {
    pos: [i32; 3],
    wait: u32,
}

struct State {
    queue: Vec<Pending>,
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
            .unwrap_or(1)
            | 1;
        Mod { state: Mutex::new(State { queue: Vec::new(), seed }) }
    }
}

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

impl Hooks for Mod {
    fn block_changed(&self, level: &dyn LevelHost, event: &mut BlockChangedEvent) {
        let s = event.new;
        // A state that already has distance=7 and persistent=false answers itself.
        let is = |k: &str, v: &str| level.state_with_property(s, k, v) == Some(s);
        if is("distance", "7") && is("persistent", "false") {
            let p = event.pos;
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let wait = (next(&mut state.seed) % MAX_DELAY as u64) as u32 + 1;
            state.queue.push(Pending { pos: [p.x, p.y, p.z], wait });
        }
    }

    fn level_tick_post(&self, host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        let due: Vec<[i32; 3]> = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.queue.is_empty() {
                return;
            }
            for p in state.queue.iter_mut() {
                p.wait = p.wait.saturating_sub(1);
            }
            let (due, rest): (Vec<_>, Vec<_>) = state.queue.drain(..).partition(|p| p.wait == 0);
            state.queue = rest;
            due.into_iter().map(|p| p.pos).collect()
        };
        if !host.on("accelerateddecay") {
            return;
        }
        for pos in due {
            let d = host.block_property(pos, "distance");
            let pe = host.block_property(pos, "persistent");
            if decays(d.as_deref(), pe.as_deref()) {
                host.run_command(&break_command(event.dimension, pos));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_cut_off_natural_leaves_decay() {
        assert!(decays(Some("7"), Some("false")));
        assert!(!decays(Some("6"), Some("false")));
        assert!(!decays(Some("7"), Some("true")));
        assert!(!decays(None, None));
    }

    #[test]
    fn break_drops_its_loot() {
        assert_eq!(
            break_command("minecraft:overworld", [3, 70, -9]),
            "execute in minecraft:overworld run setblock 3 70 -9 minecraft:air destroy"
        );
    }
}
