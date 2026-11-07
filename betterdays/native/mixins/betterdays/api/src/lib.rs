//! Better Days for Horizon: a slower clock, as Better Days 1.20.1 does with Homestead's config
//! (`speedMethod = "MINUTES"`: the day, 23500 to 12500, lasts 25 real minutes, the night, 12500
//! to 23500, 15 minutes; vanilla's are 10.8 and 9.2).
//!
//! 26.3's world clocks have a rate (`/time rate`), which the server and the clients' sky both
//! follow, so this mod only switches the overworld clock's rate at dawn and dusk. Not ported:
//! Better Days' sleep, which speeds the clock up while players sleep instead of skipping the
//! night; vanilla's skip stays. The rate is saved with the world: without this mod a world keeps
//! the last one (`/time rate 1` puts vanilla's back).
use std::sync::Mutex;

use horizon_client::mod_api::{Hooks, Host, ServerTickEvent, ServerTickPhase};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

pub const DAY_START: f64 = 23500.0;
pub const NIGHT_START: f64 = 12500.0;
/// Homestead's `daySpeedMinutes` and `nightSpeedMinutes`.
pub const DAY_MINUTES: f64 = 25.0;
pub const NIGHT_MINUTES: f64 = 15.0;

/// Clock ticks per game tick at `time` (any day): the part's length over its minutes in ticks.
pub fn speed(time: f64) -> f64 {
    let t = time.rem_euclid(24000.0);
    let night_len = DAY_START - NIGHT_START;
    if (NIGHT_START..DAY_START).contains(&t) {
        night_len / (NIGHT_MINUTES * 1200.0)
    } else {
        (24000.0 - night_len) / (DAY_MINUTES * 1200.0)
    }
}

/// The `/time rate` line for `rate`, as Java prints floats.
pub fn rate_command(rate: f64) -> String {
    format!("time rate {}", (rate * 10000.0).round() / 10000.0)
}

#[derive(Default)]
struct Mod {
    /// The rate this mod last set.
    applied: Mutex<Option<f64>>,
}

impl Hooks for Mod {
    fn server_tick(&self, host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) || !host.on("betterdays") {
            return;
        }
        let want = speed(host.day_time() as f64);
        let mut applied = self.applied.lock().unwrap_or_else(|e| e.into_inner());
        if *applied != Some(want) {
            // Fails when the clock already runs at that rate, which is as good.
            host.run_command(&rate_command(want));
            *applied = Some(want);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_and_night_take_their_minutes() {
        for (start, goal, minutes) in [(23500.0, 36500.0, 25.0), (12500.0, 23500.0, 15.0)] {
            let (mut t, mut ticks) = (start, 0u32);
            while t < goal {
                t += speed(t);
                ticks += 1;
            }
            assert!((f64::from(ticks) / 1200.0 - minutes).abs() < 0.01, "{start}: {ticks}");
        }
    }

    #[test]
    fn rates_switch_at_dawn_and_dusk() {
        assert_eq!(rate_command(speed(6000.0)), "time rate 0.4333");
        assert_eq!(rate_command(speed(23600.0)), "time rate 0.4333");
        assert_eq!(rate_command(speed(12500.0)), "time rate 0.6111");
        assert_eq!(rate_command(speed(47400.0)), "time rate 0.6111");
    }
}
