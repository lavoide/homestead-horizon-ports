//! Preferred Gamerules for Horizon: Homestead's gamerule defaults for new worlds, as the Fabric
//! mod `preferred-gamerules` 1.1.1 sets them from `preferred-gamerules.properties`:
//! `doFireTick=false` and `mobExplosionDropDecay=false` (26.3: `fire_spread_radius_around_player 0`,
//! `mob_explosion_drop_decay false`). The pack's other entries name gamerules of mods that
//! aren't ported (Secure Crops, Companion).
//!
//! The Java mod changes defaults, so only worlds created with it get them. Horizon has no "world
//! created" hook, so this applies them once per world, the first time it opens with the mod, and
//! marks the world with `data/preferredgamerules.applied`. A world's own later changes stay.
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_client::mod_api::{Hooks, Host, ServerTickEvent, WorldLifecycleEvent, WorldPhase};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::new())
}

pub const RULES: &[(&str, &str)] =
    &[("fire_spread_radius_around_player", "0"), ("mob_explosion_drop_decay", "false")];

pub const MARKER: &str = "data/preferredgamerules.applied";

pub fn commands() -> Vec<String> {
    RULES.iter().map(|(k, v)| format!("gamerule {k} {v}")).collect()
}

struct Mod {
    pending: AtomicBool,
}

impl Mod {
    fn new() -> Self {
        Mod { pending: AtomicBool::new(false) }
    }
}

impl Hooks for Mod {
    fn world_lifecycle(&self, host: &mut dyn Host, event: &mut WorldLifecycleEvent<'_>) {
        if matches!(event.phase, WorldPhase::WorldOpenReady) && host.on("preferredgamerules") {
            self.pending.store(true, Ordering::SeqCst);
        }
    }

    fn server_tick(&self, host: &mut dyn Host, _event: &mut ServerTickEvent<'_>) {
        if !self.pending.swap(false, Ordering::SeqCst) {
            return;
        }
        let Some(dir) = host.world_dir() else { return };
        let marker = dir.join(MARKER);
        if marker.exists() {
            return;
        }
        // 26.3's `/gamerule` fails when the rule already has the value ("already set"),
        // which is as good as setting it, so a failure isn't retried.
        for line in commands() {
            if !host.run_command(&line) {
                eprintln!("[preferredgamerules] `{line}` didn't run (already set?)");
            }
        }
        let _ = std::fs::create_dir_all(marker.parent().unwrap_or(&dir));
        let _ = std::fs::write(&marker, commands().join("\n") + "\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homestead_rules_in_26_3_names() {
        assert_eq!(
            commands(),
            vec!["gamerule fire_spread_radius_around_player 0", "gamerule mob_explosion_drop_decay false"]
        );
    }
}
