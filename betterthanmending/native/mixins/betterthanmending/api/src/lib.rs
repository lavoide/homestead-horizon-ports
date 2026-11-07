//! Better Than Mending for Horizon: sneak and use a damaged item with Mending to repair it with
//! your experience, as Better Than Mending 1.3.0 (legobmw99) does: 20 points of experience for
//! 40 durability when you have at least 30 points and the item lacks 40, else 2 points for 4.
//! Without 2 points the item's own use goes ahead.
//!
//! The hook API can't read a player's experience, so while a player sneaks, each server tick
//! probes it with commands: their level and points go to two scoreboard objectives, selectors
//! on those set a tag, and whether `tag <name> remove <tag>` succeeds says if the tag was there
//! (`run_command` reports a failed `execute if`/selector as success, but not a missing tag).
//! `use_item` (a player hook: no commands) decides from the last probe and queues the repair
//! for the server tick: `item modify` sets the hand's `minecraft:damage`, `experience add`
//! takes the points.
//!
//! Sneaking: the host's from the client's `movement_input` (same process), a LAN guest's from
//! `player_toggle_sneak`.
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_client::mod_api::{
    Flow, Hooks, Host, MovementInputEvent, PlayerTickEvent, PlayerToggleSneakEvent,
    ServerTickEvent, ServerTickPhase, UseItemEvent,
};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

/// What the player's experience allows, from the last probe.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Xp {
    pub at_least_2: bool,
    pub at_least_30: bool,
}

/// The repair for an item with `damage`, as the Java mod picks it: (points to take, new damage).
pub fn repair(xp: Xp, damage: u32) -> Option<(u32, u32)> {
    if damage == 0 {
        return None;
    }
    if xp.at_least_30 && damage >= 40 {
        return Some((20, damage - 40));
    }
    xp.at_least_2.then(|| (2, damage.saturating_sub(4)))
}

/// The probe for `name`: the commands to run before, and the two tags to test after.
/// A total of 30 points is level 4, or level 3 (27) and 3 points; 2 is level 1, or 2 points.
pub fn probe(name: &str) -> Vec<String> {
    let me = format!("@a[name={name}");
    vec![
        "scoreboard objectives add btm_level dummy".into(),
        "scoreboard objectives add btm_points dummy".into(),
        format!("execute as {me}] store result score @s btm_level run experience query @s levels"),
        format!("execute as {me}] store result score @s btm_points run experience query @s points"),
        format!("execute as {me},scores={{btm_level=4..}}] run tag @s add btm_30"),
        format!("execute as {me},scores={{btm_level=3,btm_points=3..}}] run tag @s add btm_30"),
        format!("execute as {me},scores={{btm_level=1..}}] run tag @s add btm_2"),
        format!("execute as {me},scores={{btm_points=2..}}] run tag @s add btm_2"),
    ]
}

/// A queued repair.
#[derive(Debug, PartialEq)]
pub struct Repair {
    pub player: String,
    pub main_hand: bool,
    pub damage: u32,
    pub points: u32,
}

impl Repair {
    pub fn commands(&self) -> [String; 2] {
        let slot = if self.main_hand { "weapon.mainhand" } else { "weapon.offhand" };
        [
            format!(
                "item modify entity {} {slot} {{\"type\":\"minecraft:set_components\",\"components\":{{\"minecraft:damage\":{}}}}}",
                self.player, self.damage
            ),
            format!("experience add {} -{} points", self.player, self.points),
        ]
    }
}

#[derive(Default)]
struct State {
    /// Players seen this session: uuid to name.
    names: HashMap<u128, String>,
    guests_sneaking: HashMap<u128, bool>,
    xp: HashMap<u128, Xp>,
    queue: Vec<Repair>,
}

#[derive(Default)]
struct Mod {
    host_sneaking: AtomicBool,
    state: Mutex<State>,
}

impl Mod {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn sneaking(&self, state: &State, uuid: u128) -> bool {
        state.guests_sneaking.get(&uuid).copied().unwrap_or_else(|| self.host_sneaking.load(Ordering::Relaxed))
    }
}

impl Hooks for Mod {
    fn movement_input(&self, _host: &dyn Host, event: &mut MovementInputEvent) {
        self.host_sneaking.store(event.sneak, Ordering::Relaxed);
    }

    fn player_toggle_sneak(&self, _host: &mut dyn Host, event: &mut PlayerToggleSneakEvent<'_>) {
        self.lock().guests_sneaking.insert(event.player, event.sneaking);
    }

    fn player_tick(&self, host: &mut dyn Host, _event: &mut PlayerTickEvent) {
        if let Some(p) = host.acting_player() {
            self.lock().names.entry(p.uuid).or_insert(p.name);
        }
    }

    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if !host.on("betterthanmending") {
            return Flow::Continue;
        }
        let stack = event.stack;
        if stack.damage == 0 || stack.enchantment_level("minecraft:mending") == 0 {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else { return Flow::Continue };
        let mut state = self.lock();
        if !self.sneaking(&state, player.uuid) {
            return Flow::Continue;
        }
        let xp = state.xp.get(&player.uuid).copied().unwrap_or_default();
        let Some((points, damage)) = repair(xp, stack.damage) else { return Flow::Continue };
        // Which hand: the main hand's stack if it is this one.
        let main_hand = host.held().is_some_and(|(item, _, _)| item == *stack.item);
        // Until the next probe, assume the points are spent.
        state.xp.insert(player.uuid, Xp::default());
        state.queue.push(Repair { player: player.name, main_hand, damage, points });
        Flow::Cancel
    }

    fn server_tick(&self, host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) || !host.on("betterthanmending") {
            return;
        }
        let (queue, sneakers) = {
            let mut state = self.lock();
            let queue = std::mem::take(&mut state.queue);
            let sneakers: Vec<(u128, String)> = state
                .names
                .iter()
                .filter(|(uuid, _)| self.sneaking(&state, **uuid))
                .map(|(u, n)| (*u, n.clone()))
                .collect();
            (queue, sneakers)
        };
        for repair in queue {
            for line in repair.commands() {
                host.run_command(&line);
            }
        }
        for (uuid, name) in sneakers {
            for line in probe(&name) {
                host.run_command(&line);
            }
            let xp = Xp {
                at_least_30: host.run_command(&format!("tag {name} remove btm_30")),
                at_least_2: host.run_command(&format!("tag {name} remove btm_2")),
            };
            self.lock().xp.insert(uuid, xp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RICH: Xp = Xp { at_least_2: true, at_least_30: true };
    const POOR: Xp = Xp { at_least_2: true, at_least_30: false };

    #[test]
    fn repairs_as_the_java_mod() {
        assert_eq!(repair(RICH, 100), Some((20, 60)));
        assert_eq!(repair(RICH, 39), Some((2, 35)));
        assert_eq!(repair(POOR, 100), Some((2, 96)));
        assert_eq!(repair(POOR, 3), Some((2, 0)));
        assert_eq!(repair(Xp::default(), 100), None);
        assert_eq!(repair(RICH, 0), None);
    }

    #[test]
    fn repair_commands() {
        let r = Repair { player: "Steve".into(), main_hand: true, damage: 60, points: 20 };
        assert_eq!(
            r.commands(),
            [
                "item modify entity Steve weapon.mainhand {\"type\":\"minecraft:set_components\",\"components\":{\"minecraft:damage\":60}}".to_string(),
                "experience add Steve -20 points".to_string(),
            ]
        );
    }

    #[test]
    fn probe_selects_by_name_and_score() {
        let p = probe("Alex");
        assert_eq!(p[4], "execute as @a[name=Alex,scores={btm_level=4..}] run tag @s add btm_30");
    }
}
