//! Armor Statues 8.0.6 (Fuzs, MPL-2.0) and its Puzzles API statue screens, for Horizon.
//!
//! Sneak and use an armor stand with an empty hand (ArmorStandInteractHelper.tryOpenArmorStatueMenu:
//! a player who may build; a sealed stand only in creative): a six-row chest menu edits the stand.
//! The menu is a server menu (`open_menu`): nothing moves on a click, the mod hears it
//! (`menu_clicked`) and changes the stand with `entity_merge_nbt`, as Armor Statues' client sends
//! `/data merge entity` on a server without the mod. Its tabs follow the screens:
//! - Style (ArmorStandStyleOptions): show name, show arms, small, invisible, no base plate, no
//!   gravity, sealed (creative only: `Invulnerable` and every equipment slot locked).
//! - Poses (ArmorStandPose.VALUES): the 29 presets by Minecraft and Vanilla Tweaks.
//! - Rotations (ArmorStandRotationsScreen): a part, then its X, Y and Z by 1, 5 or 15 degrees,
//!   each axis reset; randomize (PosePartMutator's ranges, the body kept straight), mirror, reset
//!   to the default pose, and copy / paste a pose between stands.
//! - Position (ArmorStandPositionScreen): move by 1, 4, 8 or 16 pixels along each axis, turn by
//!   5, 15 or 45 degrees.
//! - Alignments (Armor Statues' ArmorStandAlignmentsScreen): center or corner on the block, and the
//!   four held-item alignments (right arm pose and offset, made invisible and gravity-free).
//!
//! Not ported: the equipment tab (a server menu moves no items; equipping by hand is vanilla's),
//! typed position values, sliders, and the name edit box.
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use horizon_client::mod_api::{
    Compound, Flow, Hooks, Host, InteractEntityEvent, ItemStack, MenuClickedEvent, MenuClosedEvent,
    MovementInputEvent, PlayerToggleSneakEvent, ServerTickEvent, ServerTickPhase, Tag,
};
use horizon_client::text::Text;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Mod::default())
}

/// ArmorStandPose.VALUES: name, by (Minecraft or Vanilla Tweaks), and head, body, left arm, right
/// arm, left leg, right leg rotations (a part a pose leaves out is 0, 0, 0).
pub const POSES: &[(&str, &str, [[f32; 3]; 6])] = &[
    ("Default", "Minecraft", [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-10.0, 0.0, -10.0], [-15.0, 0.0, 10.0], [-1.0, 0.0, -1.0], [1.0, 0.0, 1.0]]),
    ("Solemn", "Minecraft", [[15.0, 0.0, 0.0], [0.0, 0.0, 2.0], [-30.0, 15.0, 15.0], [-60.0, -20.0, -10.0], [-1.0, 0.0, -1.0], [1.0, 0.0, 1.0]]),
    ("Athena", "Minecraft", [[-5.0, 0.0, 0.0], [0.0, 0.0, 2.0], [10.0, 0.0, -5.0], [-60.0, 20.0, -10.0], [-3.0, -3.0, -3.0], [3.0, 3.0, 3.0]]),
    ("Brandish", "Minecraft", [[-15.0, 0.0, 0.0], [0.0, 0.0, -2.0], [20.0, 0.0, -10.0], [-110.0, 50.0, 0.0], [5.0, -3.0, -3.0], [-5.0, 3.0, 3.0]]),
    ("Honor", "Minecraft", [[-15.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-110.0, 35.0, 0.0], [-110.0, -35.0, 0.0], [5.0, -3.0, -3.0], [-5.0, 3.0, 3.0]]),
    ("Entertain", "Minecraft", [[-15.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-110.0, -35.0, 0.0], [-110.0, 35.0, 0.0], [5.0, -3.0, -3.0], [-5.0, 3.0, 3.0]]),
    ("Salute", "Minecraft", [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 0.0, -5.0], [-70.0, -40.0, 0.0], [-1.0, 0.0, -1.0], [1.0, 0.0, 1.0]]),
    ("Hero", "Minecraft", [[-4.0, 67.0, 0.0], [0.0, 8.0, 0.0], [16.0, 32.0, -8.0], [-99.0, 63.0, 0.0], [0.0, -75.0, -8.0], [4.0, 63.0, 8.0]]),
    ("Riposte", "Minecraft", [[16.0, 20.0, 0.0], [0.0, 0.0, 0.0], [4.0, 8.0, 237.0], [246.0, 0.0, 89.0], [-14.0, -18.0, -16.0], [8.0, 20.0, 4.0]]),
    ("Zombie", "Minecraft", [[-10.0, 0.0, -5.0], [0.0, 0.0, 0.0], [-105.0, 0.0, 0.0], [-100.0, 0.0, 0.0], [7.0, 0.0, 0.0], [-46.0, 0.0, 0.0]]),
    ("Cancan", "Minecraft", [[-5.0, 18.0, 0.0], [0.0, 22.0, 0.0], [8.0, 0.0, -114.0], [0.0, 84.0, 111.0], [-111.0, 55.0, 0.0], [0.0, 23.0, -13.0]]),
    ("Walking", "Vanilla Tweaks", [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-20.0, 0.0, -10.0], [20.0, 0.0, 10.0], [20.0, 0.0, 0.0], [-20.0, 0.0, 0.0]]),
    ("Running", "Vanilla Tweaks", [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [40.0, 0.0, -10.0], [-40.0, 0.0, 10.0], [-40.0, 0.0, 0.0], [40.0, 0.0, 0.0]]),
    ("Pointing", "Vanilla Tweaks", [[0.0, 20.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, -10.0], [-90.0, 18.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
    ("Blocking", "Vanilla Tweaks", [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-50.0, 50.0, 0.0], [-20.0, -20.0, 0.0], [20.0, 0.0, 0.0], [-20.0, 0.0, 0.0]]),
    ("Lungeing", "Vanilla Tweaks", [[0.0, 0.0, 0.0], [15.0, 0.0, 0.0], [10.0, 0.0, -10.0], [-60.0, -10.0, 0.0], [30.0, 0.0, 0.0], [-15.0, 0.0, 0.0]]),
    ("Winning", "Vanilla Tweaks", [[-15.0, 0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 0.0, -10.0], [-120.0, -10.0, 0.0], [15.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
    ("Sitting", "Vanilla Tweaks", [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-80.0, -20.0, 0.0], [-80.0, 20.0, 0.0], [-90.0, -10.0, 0.0], [-90.0, 10.0, 0.0]]),
    ("Arabesque", "Vanilla Tweaks", [[-15.0, 0.0, 0.0], [10.0, 0.0, 0.0], [70.0, 0.0, -10.0], [-140.0, -10.0, 0.0], [75.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
    ("Cupid", "Vanilla Tweaks", [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [-75.0, 0.0, 10.0], [-90.0, -10.0, 0.0], [75.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
    ("Confident", "Vanilla Tweaks", [[-10.0, 20.0, 0.0], [-2.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 0.0, 0.0], [0.0, -10.0, -4.0], [16.0, 2.0, 10.0]]),
    ("Death", "Vanilla Tweaks", [[-85.0, 0.0, 0.0], [-90.0, 0.0, 0.0], [-90.0, -10.0, 0.0], [-90.0, 10.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
    ("Facepalm", "Vanilla Tweaks", [[45.0, -4.0, 1.0], [10.0, 0.0, 0.0], [-72.0, 24.0, 47.0], [18.0, -14.0, 0.0], [-4.0, -6.0, -2.0], [25.0, -2.0, 0.0]]),
    ("Lazing", "Vanilla Tweaks", [[14.0, -12.0, 6.0], [5.0, 0.0, 0.0], [-4.0, -20.0, -10.0], [-40.0, 20.0, 0.0], [-88.0, 46.0, 0.0], [-88.0, 71.0, 0.0]]),
    ("Confused", "Vanilla Tweaks", [[0.0, 30.0, 0.0], [0.0, 13.0, 0.0], [145.0, 22.0, -49.0], [-22.0, 31.0, 10.0], [-6.0, 0.0, 0.0], [6.0, -20.0, 0.0]]),
    ("Formal", "Vanilla Tweaks", [[4.0, 0.0, 0.0], [4.0, 0.0, 0.0], [30.0, -20.0, 21.0], [30.0, 22.0, -20.0], [0.0, 0.0, -5.0], [0.0, 0.0, 5.0]]),
    ("Sad", "Vanilla Tweaks", [[63.0, 0.0, 0.0], [10.0, 0.0, 0.0], [-5.0, 0.0, -5.0], [-5.0, 0.0, 5.0], [-5.0, 16.0, -5.0], [-5.0, -10.0, 5.0]]),
    ("Joyous", "Vanilla Tweaks", [[-11.0, 0.0, 0.0], [-4.0, 0.0, 0.0], [0.0, 0.0, -100.0], [0.0, 0.0, 100.0], [-8.0, 0.0, -60.0], [-8.0, 0.0, 60.0]]),
    ("Stargazing", "Vanilla Tweaks", [[-22.0, 25.0, 0.0], [-4.0, 10.0, 0.0], [4.0, 18.0, 0.0], [-153.0, 34.0, -3.0], [6.0, 24.0, 0.0], [-4.0, 17.0, 2.0]]),
];

/// The pose parts: NBT key, label, icon (`POSES` order).
pub const PARTS: [(&str, &str, &str); 6] = [
    ("Head", "Head", "minecraft:player_head"),
    ("Body", "Body", "minecraft:iron_chestplate"),
    ("LeftArm", "Left Arm", "minecraft:stick"),
    ("RightArm", "Right Arm", "minecraft:blaze_rod"),
    ("LeftLeg", "Left Leg", "minecraft:iron_leggings"),
    ("RightLeg", "Right Leg", "minecraft:chainmail_leggings"),
];

/// PosePartMutator's ranges per part and axis, for randomize.
pub const RANGES: [[(f32, f32); 3]; 6] = [
    [(-60.0, 60.0), (-60.0, 60.0), (-120.0, 120.0)],
    [(-30.0, 30.0), (-30.0, 30.0), (-120.0, 120.0)],
    [(-180.0, 0.0), (-45.0, 90.0), (-120.0, 120.0)],
    [(-180.0, 0.0), (-90.0, 45.0), (-120.0, 120.0)],
    [(-120.0, 120.0), (0.0, 90.0), (-120.0, 120.0)],
    [(-120.0, 120.0), (-90.0, 0.0), (-120.0, 120.0)],
];

/// ArmorStandStyleOptions: NBT key, label, description, icon.
pub const STYLES: [(&str, &str, &str, &str); 7] = [
    ("CustomNameVisible", "Show Name", "Render the statue's name tag above it's head.", "minecraft:name_tag"),
    ("ShowArms", "Show Arms", "Shows the statue's arms, so it may hold items in both hands.", "minecraft:stick"),
    ("Small", "Small", "Makes the statue half it's size like a baby mob.", "minecraft:rabbit_foot"),
    ("Invisible", "Invisible", "Makes the statue itself invisible, but still shows all equipped items.", "minecraft:glass"),
    ("NoBasePlate", "No Base Plate", "Hide the stone base plate at the statue's feet.", "minecraft:smooth_stone_slab"),
    ("NoGravity", "No Gravity", "Prevents the statue from falling down, so it may float freely.", "minecraft:feather"),
    ("Invulnerable", "Sealed", "The statue can no longer be broken, equipment cannot be changed. Disallows opening this menu in survival mode.", "minecraft:barrier"),
];
/// ArmorStandStyleOptions.ARMOR_STAND_ALL_SLOTS_DISABLED.
pub const ALL_SLOTS_DISABLED: i32 = 4144959;

/// ArmorStandAlignment: label, description, right arm pose, offset, offset when small.
pub const ALIGNMENTS: [(&str, &str, [f32; 3], [f64; 3], [f64; 3]); 4] = [
    ("Align Block On Surface", "Align an armor stand placed on a surface so that a block held by it appears on the surface.",
     [-15.0, 135.0, 0.0], [0.5725, -0.655, 0.352], [0.28625, -0.3275, 0.176]),
    ("Align Item On Surface", "Align an armor stand placed on a surface so that an item held by it appears upright on the surface.",
     [-90.0, 0.0, 0.0], [0.36, -1.41, -0.5625], [0.18, -0.705, -0.28125]),
    ("Align Item Flat On Surface", "Align an armor stand placed on a surface so that a non-tool item held by it appears flat on the surface.",
     [0.0, 0.0, 0.0], [0.385, -0.78, -0.295], [0.1925, -0.39, -0.1475]),
    ("Align Tool Flat On Surface", "Align an armor stand placed on a surface so that a tool held by it appears flat on the surface.",
     [-10.0, 0.0, -90.0], [-0.17, -1.285, -0.44], [-0.085, -0.6425, -0.22]),
];

/// ArmorStandPositionScreen.INCREMENTS, in blocks.
pub const INCREMENTS: [f64; 4] = [0.0625, 0.25, 0.5, 1.0];
pub const ROTATION_STEPS: [f32; 3] = [1.0, 5.0, 15.0];
pub const YAW_STEPS: [f32; 3] = [5.0, 15.0, 45.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Style,
    Poses,
    Rotations,
    Position,
    Alignments,
}
const TABS: [(Tab, &str, &str); 5] = [
    (Tab::Style, "Style", "minecraft:painting"),
    (Tab::Poses, "Poses", "minecraft:armor_stand"),
    (Tab::Rotations, "Rotations", "minecraft:clock"),
    (Tab::Position, "Position", "minecraft:compass"),
    (Tab::Alignments, "Alignments", "minecraft:item_frame"),
];
const CLOSE: i32 = 8;

/// What a stand holds that the menu shows and changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Stand {
    pub pose: [[f32; 3]; 6],
    pub flags: [bool; 7],
    pub pos: [f64; 3],
    pub yaw: f32,
    pub small: bool,
    pub name: Option<String>,
}

fn float(t: &Tag) -> Option<f32> {
    match t {
        Tag::Float(b) => Some(f32::from_bits(*b)),
        Tag::Double(b) => Some(f64::from_bits(*b) as f32),
        Tag::Int(i) => Some(*i as f32),
        _ => None,
    }
}
fn double(t: &Tag) -> Option<f64> {
    match t {
        Tag::Double(b) => Some(f64::from_bits(*b)),
        Tag::Float(b) => Some(f64::from(f32::from_bits(*b))),
        _ => None,
    }
}
fn list(t: Option<&Tag>) -> &[Tag] {
    match t {
        Some(Tag::List(l)) => l,
        _ => &[],
    }
}

impl Stand {
    /// From `/data get entity` (`ArmorStand.addAdditionalSaveData`); a pose part it leaves out is
    /// the default pose's, as `readPose` does.
    pub fn from_nbt(t: &Tag) -> Self {
        let mut pose = POSES[0].2;
        for (i, (key, _, _)) in PARTS.iter().enumerate() {
            let l = list(t.get("Pose").and_then(|p| p.get(key)));
            if l.len() == 3 {
                for a in 0..3 {
                    pose[i][a] = float(&l[a]).unwrap_or(pose[i][a]);
                }
            }
        }
        let mut flags = [false; 7];
        for (i, (key, ..)) in STYLES.iter().enumerate() {
            flags[i] = matches!(t.get(key), Some(Tag::Byte(b)) if *b != 0);
        }
        let p = list(t.get("Pos"));
        let pos = if p.len() == 3 {
            [double(&p[0]).unwrap_or(0.0), double(&p[1]).unwrap_or(0.0), double(&p[2]).unwrap_or(0.0)]
        } else {
            [0.0; 3]
        };
        let yaw = list(t.get("Rotation")).first().and_then(float).unwrap_or(0.0);
        let name = match t.get("CustomName") {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            Some(c @ Tag::Compound(_)) => match c.get("text") {
                Some(Tag::String(s)) => Some(s.to_string_lossy()),
                _ => None,
            },
            _ => None,
        };
        Self { pose, small: flags[2], flags, pos, yaw, name }
    }
}

fn floats(v: &[f32]) -> Tag {
    Tag::List(v.iter().map(|x| Tag::Float(x.to_bits())).collect())
}
fn compound(t: Tag) -> Compound {
    match t {
        Tag::Compound(c) => c,
        _ => Compound::new(),
    }
}
pub fn pose_nbt(pose: &[[f32; 3]; 6]) -> Compound {
    let parts: Vec<(&str, Tag)> = PARTS.iter().zip(pose).map(|((k, ..), v)| (*k, floats(v))).collect();
    compound(Tag::compound([("Pose", Tag::compound(parts))]))
}
pub fn pos_nbt(pos: [f64; 3]) -> Compound {
    compound(Tag::compound([("Pos", Tag::List(pos.iter().map(|x| Tag::Double(x.to_bits())).collect()))]))
}
pub fn yaw_nbt(yaw: f32) -> Compound {
    compound(Tag::compound([("Rotation", floats(&[yaw, 0.0]))]))
}
pub fn style_nbt(option: usize, on: bool) -> Compound {
    let mut c = compound(Tag::compound([(STYLES[option].0, Tag::Byte(i8::from(on)))]));
    if option == 6 {
        c.insert("DisabledSlots".into(), Tag::Int(if on { ALL_SLOTS_DISABLED } else { 0 }));
    }
    c
}

/// Degrees kept in -180..180, snapped to a hundredth.
pub fn wrap(v: f32) -> f32 {
    let mut v = (v + 180.0).rem_euclid(360.0) - 180.0;
    v = (v * 100.0).round() / 100.0;
    if v == -0.0 { 0.0 } else { v }
}

/// ArmorStandPose.mirror: left and right swap, Y and Z turn the other way.
pub fn mirror(p: &[[f32; 3]; 6]) -> [[f32; 3]; 6] {
    let m = |r: [f32; 3]| [r[0], wrap(-r[1]), wrap(-r[2])];
    [m(p[0]), m(p[1]), m(p[3]), m(p[2]), m(p[5]), m(p[4])]
}

/// DataSyncHandler.getLocalPosition with no pitch: `offset` (left, up, forward) from the stand.
pub fn local_offset(pos: [f64; 3], yaw: f32, offset: [f64; 3]) -> [f64; 3] {
    let r = f64::from(yaw + 90.0).to_radians();
    let (f, g) = (r.cos(), r.sin());
    [pos[0] + f * offset[2] + g * offset[0], pos[1] + offset[1], pos[2] + g * offset[2] - f * offset[0]]
}

static SEED: AtomicU64 = AtomicU64::new(0);
fn random() -> f32 {
    let mut s = SEED.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9, |d| d.as_nanos() as u64)
            | 1;
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    SEED.store(s, Ordering::Relaxed);
    (s >> 40) as f32 / (1u64 << 24) as f32
}
/// ArmorStandDataProvider.getRandomPose(true): each part in its mutator's range, the body straight.
pub fn random_pose() -> [[f32; 3]; 6] {
    let mut p = [[0.0; 3]; 6];
    for (i, part) in RANGES.iter().enumerate() {
        if i == 1 {
            continue;
        }
        for (a, (lo, hi)) in part.iter().enumerate() {
            p[i][a] = (lo + random() * (hi - lo)).round();
        }
    }
    p
}

#[derive(Clone, Debug)]
struct Session {
    window: i32,
    entity: u32,
    tab: Tab,
    part: usize,
    increment: usize,
}

#[derive(Default)]
struct Mod {
    host_sneaking: AtomicBool,
    guests_sneaking: Mutex<HashMap<u128, bool>>,
    /// Players to open the menu for at the tick's end, and the stand.
    opens: Mutex<Vec<(u128, u32)>>,
    sessions: Mutex<HashMap<u128, Session>>,
    /// Copied poses, by player.
    clipboard: Mutex<HashMap<u128, [[f32; 3]; 6]>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn button(item: &str, name: &str, lore: &[String], glint: bool) -> Option<ItemStack> {
    let mut s = ItemStack::new(item, 1);
    let d = s.data_mut();
    d.custom_name = Some(Tag::compound([("text", Tag::String(name.into())), ("italic", Tag::Byte(0))]));
    if !lore.is_empty() {
        d.other.push((
            "minecraft:lore".to_owned(),
            Tag::List(
                lore.iter()
                    .map(|l| Tag::compound([("text", Tag::String(l.as_str().into())), ("color", Tag::String("gray".into())), ("italic", Tag::Byte(0))]))
                    .collect(),
            ),
        ));
    }
    if glint {
        d.other.push(("minecraft:enchantment_glint_override".to_owned(), Tag::Byte(1)));
    }
    d.other.push((
        "minecraft:tooltip_display".to_owned(),
        Tag::compound([(
            "hidden_components",
            Tag::List(vec![Tag::String("minecraft:attribute_modifiers".into()), Tag::String("minecraft:enchantments".into())]),
        )]),
    ));
    Some(s)
}
fn filler() -> Option<ItemStack> {
    button("minecraft:gray_stained_glass_pane", " ", &[], false)
}
fn deg(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{s}°")
}
fn blocks(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The 54 slots of the menu for `stand`, as `session` shows it.
fn items(stand: &Stand, s: &Session, creative: bool) -> Vec<Option<ItemStack>> {
    let mut v: Vec<Option<ItemStack>> = (0..54).map(|_| filler()).collect();
    for (i, (tab, label, icon)) in TABS.iter().enumerate() {
        v[i] = button(icon, label, &[], *tab == s.tab);
    }
    v[CLOSE as usize] = button("minecraft:barrier", "Close", &[], false);
    match s.tab {
        Tab::Style => {
            for (i, (_, label, desc, icon)) in STYLES.iter().enumerate() {
                let on = stand.flags[i];
                let mut lore = vec![desc.to_string(), String::new(), if on { "On".into() } else { "Off".into() }];
                if i == 6 && !creative {
                    lore.push("Creative mode only".into());
                }
                v[19 + i] = button(icon, label, &lore, on);
                v[28 + i] = button(if on { "minecraft:lime_dye" } else { "minecraft:gray_dye" }, if on { "On" } else { "Off" }, &[label.to_string()], false);
            }
        }
        Tab::Poses => {
            for (i, (name, by, _)) in POSES.iter().enumerate() {
                v[9 + i] = button("minecraft:armor_stand", name, &[format!("By {by}")], false);
            }
        }
        Tab::Rotations => {
            for (i, (_, label, icon)) in PARTS.iter().enumerate() {
                let r = stand.pose[i];
                v[10 + i] = button(icon, label, &[format!("X: {}  Y: {}  Z: {}", deg(r[0]), deg(r[1]), deg(r[2]))], i == s.part);
            }
            let part = PARTS[s.part].1;
            for (a, (axis, wool)) in [("X", "minecraft:red_concrete"), ("Y", "minecraft:lime_concrete"), ("Z", "minecraft:blue_concrete")].iter().enumerate() {
                let base = 18 + 9 * a;
                let value = stand.pose[s.part][a];
                v[base] = button(wool, &format!("{part} {axis}: {}", deg(value)), &[], false);
                for (k, step) in ROTATION_STEPS.iter().enumerate() {
                    v[base + 3 - k] = button("minecraft:red_stained_glass_pane", &format!("{axis} -{}", deg(*step)), &[], false);
                    v[base + 5 + k] = button("minecraft:lime_stained_glass_pane", &format!("{axis} +{}", deg(*step)), &[], false);
                }
                v[base + 4] = button("minecraft:paper", &format!("{axis}: {}", deg(value)), &[], false);
                v[base + 8] = button("minecraft:white_stained_glass_pane", &format!("Reset {axis}"), &[format!("to {}", deg(POSES[0].2[s.part][a]))], false);
            }
            v[46] = button("minecraft:ender_eye", "Randomize", &[], false);
            v[47] = button("minecraft:glass", "Mirror", &["Left and right swap".into()], false);
            v[48] = button("minecraft:water_bucket", "Reset", &["Back to the default pose".into()], false);
            v[50] = button("minecraft:writable_book", "Copy", &["Copy this pose".into()], false);
            v[51] = button("minecraft:written_book", "Paste", &["Paste the copied pose".into()], false);
        }
        Tab::Position => {
            for (i, inc) in INCREMENTS.iter().enumerate() {
                let px = (inc * 16.0).round() as i32;
                v[11 + i] = button("minecraft:iron_nugget", &format!("Move By: {px} Pixel(s)"), &[format!("{} Block(s)", blocks(*inc))], i == s.increment);
            }
            let inc = INCREMENTS[s.increment];
            for (a, axis) in ["X", "Y", "Z"].iter().enumerate() {
                let base = 18 + 9 * a;
                v[base + 2] = button("minecraft:red_stained_glass_pane", &format!("{axis} -{} Pixel(s)", (inc * 16.0).round()), &[], false);
                v[base + 4] = button("minecraft:paper", &format!("{axis}-Position: {}", blocks(stand.pos[a])), &[], false);
                v[base + 6] = button("minecraft:lime_stained_glass_pane", &format!("{axis} +{} Pixel(s)", (inc * 16.0).round()), &[], false);
            }
            for (k, step) in YAW_STEPS.iter().enumerate() {
                v[48 - k] = button("minecraft:red_stained_glass_pane", &format!("Rotation -{}", deg(*step)), &[], false);
                v[50 + k] = button("minecraft:lime_stained_glass_pane", &format!("Rotation +{}", deg(*step)), &[], false);
            }
            v[49] = button("minecraft:compass", &format!("Rotation: {}", deg(stand.yaw)), &[], false);
        }
        Tab::Alignments => {
            v[19] = button("minecraft:oak_pressure_plate", "Align Centered", &["Align an armor stand in the center of the block position it is placed on.".into()], false);
            v[20] = button("minecraft:stone_button", "Align Cornered", &["Align an armor stand at the corner of the block position it is placed on.".into()], false);
            for (i, (label, desc, ..)) in ALIGNMENTS.iter().enumerate() {
                let icon = ["minecraft:grass_block", "minecraft:apple", "minecraft:map", "minecraft:iron_pickaxe"][i];
                v[22 + i] = button(icon, label, &[desc.to_string()], false);
            }
        }
    }
    v
}

/// One change a click makes.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    None,
    Tab(Tab),
    Part(usize),
    Increment(usize),
    Close,
    Merge(Vec<Compound>),
    Copy,
    Paste,
}

/// What a left or right click on `slot` does to `stand`.
fn click(stand: &Stand, s: &Session, slot: i32, creative: bool) -> Change {
    if slot == CLOSE {
        return Change::Close;
    }
    if (0..TABS.len() as i32).contains(&slot) {
        return Change::Tab(TABS[slot as usize].0);
    }
    let slot = slot as usize;
    match s.tab {
        Tab::Style => {
            let i = match slot {
                19..=25 => slot - 19,
                28..=34 => slot - 28,
                _ => return Change::None,
            };
            if i == 6 && !creative {
                return Change::None;
            }
            Change::Merge(vec![style_nbt(i, !stand.flags[i])])
        }
        Tab::Poses => match slot.checked_sub(9).and_then(|i| POSES.get(i)) {
            Some((.., pose)) => Change::Merge(vec![pose_nbt(pose)]),
            None => Change::None,
        },
        Tab::Rotations => {
            if (10..16).contains(&slot) {
                return Change::Part(slot - 10);
            }
            let mut pose = stand.pose;
            match slot {
                46 => pose = random_pose(),
                47 => pose = mirror(&pose),
                48 => pose = POSES[0].2,
                50 => return Change::Copy,
                51 => return Change::Paste,
                18..=44 => {
                    let (a, col) = ((slot - 18) / 9, (slot - 18) % 9);
                    let r = &mut pose[s.part][a];
                    match col {
                        1..=3 => *r = wrap(*r - ROTATION_STEPS[3 - col]),
                        5..=7 => *r = wrap(*r + ROTATION_STEPS[col - 5]),
                        8 => *r = POSES[0].2[s.part][a],
                        _ => return Change::None,
                    }
                }
                _ => return Change::None,
            }
            Change::Merge(vec![pose_nbt(&pose)])
        }
        Tab::Position => {
            if (11..15).contains(&slot) {
                return Change::Increment(slot - 11);
            }
            let inc = INCREMENTS[s.increment];
            match slot {
                18..=44 => {
                    let (a, col) = ((slot - 18) / 9, (slot - 18) % 9);
                    let mut pos = stand.pos;
                    match col {
                        2 => pos[a] -= inc,
                        6 => pos[a] += inc,
                        _ => return Change::None,
                    }
                    Change::Merge(vec![pos_nbt(pos)])
                }
                46..=48 => Change::Merge(vec![yaw_nbt(wrap(stand.yaw - YAW_STEPS[48 - slot]))]),
                50..=52 => Change::Merge(vec![yaw_nbt(wrap(stand.yaw + YAW_STEPS[slot - 50]))]),
                _ => Change::None,
            }
        }
        Tab::Alignments => match slot {
            19 | 20 => {
                let f = |v: f64| v.floor();
                let half = if slot == 19 { 0.5 } else { 0.0 };
                Change::Merge(vec![pos_nbt([f(stand.pos[0]) + half, f(stand.pos[1]), f(stand.pos[2]) + half])])
            }
            22..=25 => {
                let (_, _, arm, offset, small) = ALIGNMENTS[slot - 22];
                let mut pose = [[0.0; 3]; 6];
                pose[3] = arm;
                let to = local_offset(stand.pos, stand.yaw, if stand.small { small } else { offset });
                Change::Merge(vec![style_nbt(3, true), style_nbt(5, true), pose_nbt(&pose), pos_nbt(to)])
            }
            _ => Change::None,
        },
    }
}

impl Mod {
    fn sneaking(&self, uuid: u128) -> bool {
        lock(&self.guests_sneaking)
            .get(&uuid)
            .copied()
            .unwrap_or_else(|| self.host_sneaking.load(Ordering::Relaxed))
    }
}

fn game_mode(host: &dyn Host, uuid: u128) -> i32 {
    host.server_players().into_iter().find(|p| p.uuid == uuid).map_or(0, |p| p.game_mode)
}

fn read(host: &mut dyn Host, id: u32) -> Option<Stand> {
    let e = host.entity(id)?;
    if host.entity_kind(e).as_deref() != Some("minecraft:armor_stand") {
        return None;
    }
    host.entity_nbt(e).map(|c| Stand::from_nbt(&Tag::Compound(c)))
}

fn title(stand: &Stand) -> Text {
    Text::literal(stand.name.clone().unwrap_or_else(|| "Armor Stand".to_owned()))
}

impl Hooks for Mod {
    fn movement_input(&self, _host: &dyn Host, event: &mut MovementInputEvent) {
        self.host_sneaking.store(event.sneak, Ordering::Relaxed);
    }

    fn player_toggle_sneak(&self, _host: &mut dyn Host, event: &mut PlayerToggleSneakEvent<'_>) {
        lock(&self.guests_sneaking).insert(event.player, event.sneaking);
    }

    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind != "minecraft:armor_stand" || event.stack.is_some() || !host.on("armorstatues") {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else { return Flow::Continue };
        // Abilities.mayBuild: not in adventure or spectator.
        if !matches!(player.game_mode, 0 | 1) || !self.sneaking(player.uuid) {
            return Flow::Continue;
        }
        let Ok(id) = u32::try_from(event.entity) else { return Flow::Continue };
        if let Some(stand) = read(host, id)
            && stand.flags[6]
            && player.game_mode != 1
        {
            return Flow::Continue;
        }
        lock(&self.opens).push((player.uuid, id));
        Flow::Cancel
    }

    fn server_tick(&self, host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let opens = std::mem::take(&mut *lock(&self.opens));
        for (uuid, id) in opens {
            let Some(stand) = read(host, id) else { continue };
            let mut session = Session { window: -1, entity: id, tab: Tab::Style, part: 0, increment: 0 };
            if let Some(old) = lock(&self.sessions).get(&uuid) {
                session.tab = old.tab;
                session.part = old.part;
                session.increment = old.increment;
            }
            let creative = game_mode(host, uuid) == 1;
            let shown = items(&stand, &session, creative);
            if let Some(window) = host.open_menu(uuid, 6, title(&stand), shown) {
                session.window = window;
                lock(&self.sessions).insert(uuid, session);
            }
        }
    }

    fn menu_clicked(&self, host: &mut dyn Host, event: &mut MenuClickedEvent) {
        let Some(mut session) = lock(&self.sessions).get(&event.uuid).cloned() else { return };
        if session.window != event.window {
            return;
        }
        // PICKUP and QUICK_MOVE with either button; a drag, swap or throw does nothing.
        if !matches!(event.mode, 0 | 1) || !(0..54).contains(&event.slot) {
            return;
        }
        let Some(stand) = read(host, session.entity) else {
            host.close_menu(event.uuid);
            return;
        };
        let creative = game_mode(host, event.uuid) == 1;
        if stand.flags[6] && !creative {
            host.close_menu(event.uuid);
            return;
        }
        match click(&stand, &session, event.slot, creative) {
            Change::None => return,
            Change::Close => {
                host.close_menu(event.uuid);
                return;
            }
            Change::Tab(t) => session.tab = t,
            Change::Part(p) => session.part = p,
            Change::Increment(i) => session.increment = i,
            Change::Copy => {
                lock(&self.clipboard).insert(event.uuid, stand.pose);
            }
            Change::Paste => {
                let copied = lock(&self.clipboard).get(&event.uuid).copied();
                if let (Some(pose), Some(e)) = (copied, host.entity(session.entity)) {
                    host.entity_merge_nbt(e, pose_nbt(&pose));
                }
            }
            Change::Merge(parts) => {
                if let Some(e) = host.entity(session.entity) {
                    for nbt in parts {
                        host.entity_merge_nbt(e, nbt);
                    }
                }
            }
        }
        let stand = read(host, session.entity).unwrap_or(stand);
        host.set_menu_items(event.uuid, items(&stand, &session, creative));
        lock(&self.sessions).insert(event.uuid, session);
    }

    fn menu_closed(&self, _host: &mut dyn Host, event: &mut MenuClosedEvent) {
        let mut sessions = lock(&self.sessions);
        if sessions.get(&event.uuid).is_some_and(|s| s.window == event.window) {
            // Keep the tab, part and increment for the next opening.
            if let Some(s) = sessions.get_mut(&event.uuid) {
                s.window = -1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stand() -> Stand {
        Stand { pose: POSES[0].2, flags: [false; 7], pos: [10.3, 64.0, -2.7], yaw: 0.0, small: false, name: None }
    }
    fn session(tab: Tab) -> Session {
        Session { window: 1, entity: 5, tab, part: 3, increment: 0 }
    }

    #[test]
    fn all_presets_are_there() {
        assert_eq!(POSES.len(), 29);
        assert_eq!(POSES[0].0, "Default");
    }

    #[test]
    fn mirror_swaps_sides_and_flips_y_z() {
        let m = mirror(&POSES[1].2);
        assert_eq!(m[2], [-60.0, 20.0, 10.0]);
        assert_eq!(m[3], [-30.0, -15.0, -15.0]);
        assert_eq!(mirror(&m), POSES[1].2);
    }

    #[test]
    fn wrap_keeps_degrees_in_range() {
        assert_eq!(wrap(185.0), -175.0);
        assert_eq!(wrap(-181.0), 179.0);
        assert_eq!(wrap(12.344), 12.34);
    }

    #[test]
    fn nbt_round_trip() {
        let mut c = pose_nbt(&POSES[5].2);
        c.extend(pos_nbt([1.5, 2.0, 3.25]));
        c.extend(yaw_nbt(45.0));
        c.extend(style_nbt(1, true));
        let s = Stand::from_nbt(&Tag::Compound(c));
        assert_eq!(s.pose, POSES[5].2);
        assert_eq!(s.pos, [1.5, 2.0, 3.25]);
        assert_eq!(s.yaw, 45.0);
        assert!(s.flags[1] && !s.flags[0]);
    }

    #[test]
    fn missing_pose_parts_are_the_default() {
        let s = Stand::from_nbt(&Tag::compound([("Pose", Tag::compound([("Head", floats(&[5.0, 0.0, 0.0]))]))]));
        assert_eq!(s.pose[0], [5.0, 0.0, 0.0]);
        assert_eq!(s.pose[2], [-10.0, 0.0, -10.0]);
    }

    #[test]
    fn rotation_buttons_step_the_selected_part() {
        let st = stand();
        let Change::Merge(m) = click(&st, &session(Tab::Rotations), 18 + 7, false) else { panic!() };
        let s = Stand::from_nbt(&Tag::Compound(m[0].clone()));
        assert_eq!(s.pose[3], [0.0, 0.0, 10.0]);
        let Change::Merge(m) = click(&st, &session(Tab::Rotations), 27 + 1, false) else { panic!() };
        assert_eq!(Stand::from_nbt(&Tag::Compound(m[0].clone())).pose[3], [-15.0, -15.0, 10.0]);
    }

    #[test]
    fn sealed_needs_creative() {
        assert_eq!(click(&stand(), &session(Tab::Style), 25, false), Change::None);
        let Change::Merge(m) = click(&stand(), &session(Tab::Style), 25, true) else { panic!() };
        assert!(matches!(Tag::Compound(m[0].clone()).get("DisabledSlots"), Some(Tag::Int(ALL_SLOTS_DISABLED))));
    }

    #[test]
    fn centering_and_offsets() {
        let Change::Merge(m) = click(&stand(), &session(Tab::Alignments), 19, false) else { panic!() };
        assert_eq!(Stand::from_nbt(&Tag::Compound(m[0].clone())).pos, [10.5, 64.0, -2.5]);
        // Yaw 0 faces south (+z): forward is +z, left is +x.
        let p = local_offset([0.0; 3], 0.0, [1.0, 0.0, 2.0]);
        assert!((p[0] - 1.0).abs() < 1e-9 && (p[2] - 2.0).abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn random_poses_stay_in_range() {
        for _ in 0..50 {
            let p = random_pose();
            assert_eq!(p[1], [0.0; 3]);
            for (i, part) in RANGES.iter().enumerate() {
                if i == 1 {
                    continue;
                }
                for (a, (lo, hi)) in part.iter().enumerate() {
                    assert!(p[i][a] >= *lo && p[i][a] <= *hi);
                }
            }
        }
    }

    #[test]
    fn every_tab_has_54_slots() {
        for (t, ..) in TABS {
            assert_eq!(items(&stand(), &session(t), true).len(), 54);
        }
    }
}
