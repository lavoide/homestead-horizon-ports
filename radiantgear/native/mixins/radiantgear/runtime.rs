// Radiant Gear 2.1.6 (Illusive Soulworks, LGPL-3.0-or-later): worn trinkets that glow light up around
// the player with Horizon Extras' dynamic lights, as Radiant Gear hands them to LambDynamicLights:
// the item's dynamic light data (ITEM_LIGHTS, from the jars' assets/*/dynamiclights/item/*.json,
// written by tools/trinkets_port.py), else its block's glow. A water-sensitive one goes dark with
// the player's eyes under water. Injected as `crate::game::radiantgear`.
use super::*;
use super::radiantgear_data::ITEM_LIGHTS;
use horizon_world::attachments::{AttachmentCodec, AttachmentError, AttachmentType, Holder};
use horizon_world::nbt::{Compound, Tag};

fn same(c: &Compound) -> std::result::Result<Compound, AttachmentError> {
    Ok(c.clone())
}
fn empty() -> Compound {
    Compound::new()
}
/// native/trinkets' worn trinkets (slot id -> item NBT): the same key and type read the same
/// value, so this bundle needs no code of native/trinkets (nothing is worn without it).
static WORN: AttachmentType<Compound> = AttachmentType {
    namespace: "trinkets",
    name: "worn",
    default: empty,
    codec: Some(AttachmentCodec { encode: same, decode: same }),
    copy_on_death: true,
    sync: true,
};

fn own_uuid(game: &Game<'_>) -> Option<u128> {
    match &game.remote {
        Some(r) => Some(r.profile()),
        None => game.level.player_uuid.map(|i| i.iter().fold(0u128, |a, &v| a << 32 | u128::from(v as u32))),
    }
}

/// The worn stacks where only `&Game` is at hand, read from the holder's saved form without
/// inserting anything; a remote client reads its synced copy.
fn worn_view(game: &Game<'_>) -> Vec<(String, ItemStack)> {
    let Some(uuid) = game.lan_acting_uuid.or_else(|| own_uuid(game)) else { return Vec::new() };
    let att = if game.native_authoritative() {
        if game.lan_acting_uuid == Some(uuid) {
            Some(&game.player_attachments)
        } else {
            game.guest_attachments.get(&uuid)
        }
    } else {
        game.remote.as_ref().and_then(|r| r.attachments.get(&Holder::Player(uuid)))
    };
    let nbt = att.map(|a| a.to_nbt());
    let value = nbt
        .as_ref()
        .and_then(|n| match n.get(&"trinkets".into()) {
            Some(Tag::Compound(ns)) => Some(ns),
            _ => None,
        })
        .and_then(|ns| match ns.get(&"worn".into()) {
            Some(Tag::Compound(e)) => Some(e),
            _ => None,
        })
        .and_then(|e| match e.get(&"value".into()) {
            Some(Tag::Compound(v)) => Some(v),
            _ => None,
        });
    value
        .map(|v| {
            v.iter()
                .filter_map(|(k, t)| horizon_client::save::item_from_nbt(t).map(|s| (String::from_utf16_lossy(&k.0), s)))
                .collect()
        })
        .unwrap_or_default()
}

fn eyes_in_water(game: &Game<'_>, p: &horizon_client::motion::PlayerMotion) -> bool {
    let eyes = [p.position.x.floor() as i32, (p.position.y + 1.62).floor() as i32, p.position.z.floor() as i32];
    let name = &game.res.facts().get(game.world.block(eyes[0], eyes[1], eyes[2])).name;
    matches!(&**name, "minecraft:water" | "minecraft:bubble_column" | "minecraft:kelp" | "minecraft:kelp_plant"
        | "minecraft:seagrass" | "minecraft:tall_seagrass")
}

/// The brightest light of the worn stacks: (level, item).
fn worn_light(game: &Game<'_>, worn: &[(String, ItemStack)], wet: bool) -> Option<(u8, String)> {
    let lum = game.luminance();
    worn.iter()
        .filter_map(|(_, s)| {
            let level = match ITEM_LIGHTS.iter().find(|l| l.0 == &*s.item) {
                Some(&(_, level, sensitive)) => if sensitive && wet { 0 } else { level },
                None => lum.of(&s.item),
            };
            (level > 0).then(|| (level, s.item.to_string()))
        })
        .max_by_key(|(level, _)| *level)
}

/// GameRenderer's dynamic lights with Radiant Gear: the window's own player's worn light.
/// Horizon's `gather_dynamic_lights` with the worn trinket's light added to its sources
/// (the nearest MAX are kept, as before).
pub(crate) fn gather_with_worn(game: &mut Game<'_>, camera: glam::DVec3, alpha: f64) -> core::ops::ControlFlow<()> {
    if !game.settings.extras.dynamic_lights {
        return core::ops::ControlFlow::Continue(());
    }
    let Some(p) = game.player.as_ref().filter(|p| !p.spectator) else { return core::ops::ControlFlow::Continue(()) };
    let Some((level, _item)) = worn_light(game, &worn_view(game), eyes_in_water(game, p)) else {
        return core::ops::ControlFlow::Continue(());
    };
    let feet = game.previous_position.lerp(p.position, game.clock.camera_fraction());
    let worn = horizon_client::dynamic_light::Source { at: glam::DVec3::new(feet.x, feet.y + 0.9, feet.z), level };
    let mut out = std::mem::take(&mut game.dynamic_lights);
    horizon_client::dynamic_light::select_iter(
        game.dynamic_light_sources(alpha).map(|(s, _)| s).chain(std::iter::once(worn)),
        camera,
        &mut out,
    );
    game.dynamic_lights = out;
    core::ops::ControlFlow::Break(())
}

