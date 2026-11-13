//! Burnt Offering — {B} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Add X mana in any combination of {B} and/or {R}, where X is the sacrificed creature's mana value.
//! Set: ICE #116 — Ice Age | Scryfall ID: 1dae52a2-3af7-4b97-9d2e-2448b7c413fb | Oracle ID: 86eb30a0-0beb-42db-9ddf-cf8be6c99dd3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BURNT_OFFERING,
    oracle_id = "86eb30a0-0beb-42db-9ddf-cf8be6c99dd3",
    scryfall_id = "1dae52a2-3af7-4b97-9d2e-2448b7c413fb",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Burnt Offering",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(&[Effect::mana_combination(
        &[ManaColor::Black, ManaColor::Red],
        Amount::SacrificedManaValue,
    )])],
);
