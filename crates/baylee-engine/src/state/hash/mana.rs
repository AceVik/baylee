//! Canonical mana provenance for loop detection: production IDs are not rules.

use super::{GameState, Hasher, ObjectId, hash_damage_source, moves_since};
use baylee_core::mana::{ManaColor, ManaPool, RestrictedMana};
use std::hash::Hash;

fn unit_key(state: &GameState, unit: RestrictedMana, position: &impl Fn(ObjectId) -> u32) -> u64 {
    let mut h = Hasher::new();
    unit.color.hash(&mut h);
    unit.flags.hash(&mut h);
    let provenance = state.restriction_info.get(&unit.restriction.0);
    h.boolean(provenance.is_some());
    if let Some(provenance) = provenance {
        h.u32(position(provenance.context.source));
        provenance.context.text.hash(&mut h);
        provenance.controller.hash(&mut h);
        provenance.restriction.hash(&mut h);
        provenance.chosen_subtype.hash(&mut h);
        provenance.source_watermark.hash(&mut h);
        if let Some(version) = provenance.source_version {
            h.u8(moves_since(
                state.object(provenance.context.source),
                version,
            ));
            hash_damage_source(&mut h, state, provenance.context.source, version);
        } else {
            h.u8(u8::MAX);
        }
    }
    h.finish()
}

fn rows(h: &mut Hasher, mut values: Vec<(u64, u64)>) {
    values.sort_unstable_by_key(|&(key, _)| key);
    let mut grouped: Vec<(u64, u64)> = Vec::with_capacity(values.len());
    for (key, amount) in values {
        if let Some(last) = grouped.last_mut().filter(|last| last.0 == key) {
            last.1 += amount;
        } else {
            grouped.push((key, amount));
        }
    }
    grouped.hash(h);
}

pub(super) fn pool(
    h: &mut Hasher,
    state: &GameState,
    pool: &ManaPool,
    position: &impl Fn(ObjectId) -> u32,
) {
    for color in ManaColor::ALL {
        h.u32(pool.available(color));
        h.u32(pool.snow_available(color));
    }
    for units in [pool.restricted(), pool.ridden()] {
        rows(
            h,
            units
                .iter()
                .map(|unit| (unit_key(state, *unit, position), u64::from(unit.amount)))
                .collect(),
        );
    }
}

pub(super) fn riders(
    h: &mut Hasher,
    state: &GameState,
    paid: &crate::object::PaidRecord,
    position: &impl Fn(ObjectId) -> u32,
) {
    rows(
        h,
        paid.spend_riders
            .iter()
            .map(|(unit, source, rider)| {
                let mut key = Hasher::new();
                key.u64(unit_key(state, *unit, position));
                key.u32(position(*source));
                rider.hash(&mut key);
                (key.finish(), u64::from(unit.amount))
            })
            .collect(),
    );
}
