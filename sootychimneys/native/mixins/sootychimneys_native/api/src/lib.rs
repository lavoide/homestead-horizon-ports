//! Sooty Chimneys 1.3.4 (mortuusars), the ticking half: each chimney's block entity
//! (ChimneyBlockEntity) keeps its states, dirties, and smokes. Homestead's common config:
//! SmokeFlow 0.75, DirtyChance 0.05, Wind true, WindStrength 1.0. The smoke is the client's
//! particleTick on Java; here the server sends it (`particle ... force`), with one wind for the
//! server (Wind.update, ticked by the server instead of each client).
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use horizon_client::mod_api::{
    BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent, Compound, Hooks, Host, ServerTickEvent,
};

pub fn hooks() -> Box<dyn Hooks> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(7, |d| d.as_nanos() as u64) | 1;
    Box::new(Mod { state: Mutex::new(State { rng: seed, queue: Vec::new(), wind_angle: 0.0, wind_strength: 0.0, last_tick: None }) })
}

const ENTITY: &str = "sootychimneys:chimney";
const SMOKE_FLOW: f64 = 0.75;
const DIRTY_CHANCE: f64 = 0.05;
/// A block's chance to be picked by a random tick in a tick: randomTickSpeed 3 of 4096.
const RANDOM_TICK: f64 = 3.0 / 4096.0;
const WIND_STRENGTH: f32 = 1.0;

/// ChimneyTypes' SmokeProperties: origin (x, y, z), spread (x, y, z), intensity, speed.
struct Smoke {
    origin: [f64; 3],
    spread: [f64; 3],
    intensity: f32,
    speed: f64,
}

fn smoke_of(kind: &str) -> Option<Smoke> {
    let s = |origin, spread, intensity, speed| Some(Smoke { origin, spread, intensity, speed });
    match kind {
        "brick" => s([0.5, 1.25, 0.5], [0.25, 0.1, 0.25], 1.0, 1.2),
        "cobblestone" => s([0.5, 1.25, 0.5], [0.15, 0.1, 0.15], 1.0, 1.0),
        "stone_brick" => s([0.5, 1.2, 0.5], [0.025, 0.05, 0.025], 0.65, 1.0),
        "mud_brick" => s([0.5, 1.25, 0.5], [0.15, 0.1, 0.15], 1.0, 1.1),
        "iron" => s([0.5, 1.2, 0.5], [0.05, 0.05, 0.05], 0.65, 1.35),
        "copper" => s([0.5, 1.3, 0.5], [0.025, 0.05, 0.025], 0.55, 1.2),
        "terracotta" => s([0.5, 0.8, 0.5], [0.02, 0.05, 0.02], 0.3, 0.65),
        _ => None,
    }
}

/// `sootychimneys:<dirty_>?<kind>_chimney` → (kind, dirty).
pub fn parse(block: &str) -> Option<(&str, bool)> {
    let rest = block.strip_prefix("sootychimneys:")?.strip_suffix("_chimney")?;
    Some(match rest.strip_prefix("dirty_") {
        Some(kind) => (kind, true),
        None => (rest, false),
    })
}

struct State {
    rng: u64,
    queue: Vec<String>,
    wind_angle: f64,
    wind_strength: f32,
    last_tick: Option<i64>,
}

impl State {
    fn next(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn float(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn double(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { (self.next() % u64::from(n)) as u32 }
    }
    /// RandomOffset.offset: within (-range, range).
    fn offset(&mut self, v: f64, range: f64) -> f64 {
        if range <= 0.0 { v } else { v + (self.double() * 2.0 - 1.0) * range }
    }
}

/// TimeOfDay.fromSunAngle of the day time: 0 morning, 1 day, 2 evening, 3 night.
fn time_of_day(day_time: i64) -> u8 {
    // DimensionType.timeOfDay → the sun angle in degrees.
    let d = (day_time.rem_euclid(24000) as f64) / 24000.0 - 0.25;
    let f = d - d.floor();
    let g = 0.5 - (f * std::f64::consts::PI).cos() / 2.0;
    let degrees = (f * 2.0 + g) / 3.0 * 360.0;
    if degrees <= 290.0 && degrees > 270.0 {
        0
    } else if degrees <= 270.0 && degrees > 95.0 {
        3
    } else if degrees <= 95.0 && degrees > 75.0 {
        2
    } else {
        1
    }
}

/// Wind.getWindState: (min, max) strength for the weather, the time of day and the day.
fn wind_state(day_time: i64, rain: f32, thunder: f32) -> (f32, f32) {
    const CALM: (f32, f32) = (0.0, 0.05);
    const BREEZE: (f32, f32) = (0.05, 0.15);
    const WINDY: (f32, f32) = (0.15, 0.5);
    const STORMY: (f32, f32) = (0.5, 1.0);
    let day = day_time / 24000;
    let tod = time_of_day(day_time);
    if thunder > 0.0 {
        STORMY
    } else if rain > 0.0 {
        if tod != 2 && day % 3 > 0 { WINDY } else { BREEZE }
    } else if tod == 1 {
        if day % 5 > 2 { BREEZE } else { CALM }
    } else if day % 4 == 0 {
        BREEZE
    } else {
        CALM
    }
}

struct Mod {
    state: Mutex<State>,
}

impl Hooks for Mod {
    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent<'_>) -> Option<(String, Compound)> {
        parse(event.block).map(|_| (ENTITY.to_owned(), Compound::new()))
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        if parse(event.block).is_some() {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let block = host.block(pos);
        let Some((kind, dirty)) = parse(&block) else { return };
        let Some(smoke) = smoke_of(kind) else { return };
        let dim = host.dimension();
        let [x, y, z] = pos;
        let above = host.block([x, y + 1, z]);
        let below = host.block([x, y - 1, z]);
        let prop = |k: &str| host.block_property(pos, k).is_some_and(|v| v == "true");
        let (lit, blocked, stacked) = (prop("lit"), prop("blocked"), prop("stacked"));
        // ChimneyBlock.updateState: lit without a redstone signal, stacked under a chimney.
        let want_lit = host.neighbor_signal(dim, pos) == 0;
        let want_stacked = parse(&above).is_some();
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if dirty && want_stacked {
            // A stacked chimney is never dirty.
            let clean = format!("sootychimneys:{kind}_chimney");
            st.queue.push(format!(
                "execute in {dim} run setblock {x} {y} {z} {clean}[lit={want_lit},blocked={blocked},stacked=true]"
            ));
            st.queue.push(format!(
                "execute in {dim} run particle minecraft:smoke {} {} {} 0.2 0.2 0.2 0.02 6 force",
                f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5
            ));
            return;
        }
        drop(st);
        if lit != want_lit {
            host.set_block_property(pos, "lit", if want_lit { "true" } else { "false" });
        }
        if stacked != want_stacked {
            host.set_block_property(pos, "stacked", if want_stacked { "true" } else { "false" });
        }
        // shouldEmitSmoke (no block is in sootychimneys:smoke_blocking).
        if !want_lit || blocked || want_stacked {
            return;
        }
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // randomTick: a clean chimney that smokes gets dirty.
        if !dirty && st.double() < RANDOM_TICK * DIRTY_CHANCE {
            st.queue.push(format!(
                "execute in {dim} run setblock {x} {y} {z} sootychimneys:dirty_{kind}_chimney[lit=true,blocked=false,stacked=false]"
            ));
        }
        // ChimneyBlockEntity.particleTick and ChimneyBlock.emitParticle.
        if st.double() >= SMOKE_FLOW || st.float() > smoke.intensity {
            return;
        }
        let signal = parse(&below).is_some() && host.block_property([x, y - 1, z], "stacked").is_some_and(|v| v == "true");
        let particle = if signal { "minecraft:campfire_signal_smoke" } else { "minecraft:campfire_cosy_smoke" };
        let strength = st.wind_strength * 0.1 * WIND_STRENGTH;
        let rad = st.wind_angle.to_radians();
        let mut vx = rad.cos() * f64::from(strength);
        let vy = 0.05 * smoke.speed;
        let mut vz = rad.sin() * f64::from(strength);
        vx += f64::from((st.float() * strength - strength / 2.0) * 0.2);
        vz += f64::from((st.float() * strength - strength / 2.0) * 0.2);
        let max = (4.0 * smoke.intensity.max(0.5)) as u32;
        let (cx, cy, cz) = (
            f64::from(x) + smoke.origin[0],
            f64::from(y) + smoke.origin[1],
            f64::from(z) + smoke.origin[2],
        );
        for _ in 0..st.below(max) {
            let px = st.offset(cx, smoke.spread[0]);
            let py = st.offset(cy, smoke.spread[1]);
            let pz = st.offset(cz, smoke.spread[2]);
            st.queue.push(format!(
                "execute in {dim} run particle {particle} {px:.3} {py:.3} {pz:.3} {vx:.4} {vy:.4} {vz:.4} 1 0 force"
            ));
        }
    }

    fn server_tick(&self, host: &mut dyn Host, _event: &mut ServerTickEvent<'_>) {
        let day_time = host.day_time();
        let (rain, thunder) = host.weather();
        let pending = {
            let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
            // Wind.update once a game tick (the server ticks twice here: start and end).
            if st.last_tick != Some(day_time) {
                st.last_tick = Some(day_time);
                let (min, max) = wind_state(day_time, rain, thunder);
                let add = f64::from(max) * (f64::from(st.below(2)) - 1.0);
                let target = min + st.float() * (max - min);
                st.wind_strength += 0.1 * (target - st.wind_strength);
                st.wind_angle = (st.wind_angle + add) % 360.0;
            }
            std::mem::take(&mut st.queue)
        };
        for line in pending {
            if !host.run_command(&line) {
                eprintln!("[sootychimneys] command failed: {line}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(parse("sootychimneys:dirty_mud_brick_chimney"), Some(("mud_brick", true)));
        assert_eq!(parse("sootychimneys:iron_chimney"), Some(("iron", false)));
        assert_eq!(parse("minecraft:stone"), None);
        assert!(smoke_of("terracotta").is_some());
    }

    #[test]
    fn times_of_day() {
        assert_eq!(time_of_day(6000), 1);
        assert_eq!(time_of_day(18000), 3);
        assert_eq!(wind_state(6000, 0.0, 1.0), (0.5, 1.0));
    }
}
