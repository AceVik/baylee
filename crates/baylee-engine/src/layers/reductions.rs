//! A reduction whose lower bound belongs to this effect's timestamp only.

use crate::object::Characteristics;

pub(super) fn reduce_pt(
    characteristics: &mut Characteristics,
    count: i16,
    minimum_toughness: Option<i16>,
) {
    if let Some(power) = &mut characteristics.power {
        *power = power.saturating_sub(count);
    }
    if let Some(toughness) = &mut characteristics.toughness {
        // The minimum bounds this reduction, not the resulting creature.
        // In particular, a later timestamp can reduce toughness below it.
        let reduction = minimum_toughness.map_or(count, |minimum| {
            count.min(toughness.saturating_sub(minimum))
        });
        *toughness = toughness.saturating_sub(reduction);
    }
}
