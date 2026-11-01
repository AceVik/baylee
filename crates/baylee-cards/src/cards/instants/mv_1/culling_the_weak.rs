//! Culling the Weak — {B} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Add {B}{B}{B}{B}.
//! Set: EXO #55 — Exodus | Scryfall ID: 50c33f18-0a5c-4e46-ab0d-6e450915594f | Oracle ID: 6385fd0d-256d-4400-9319-afa8170024db
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CULLING_THE_WEAK,
    oracle_id = "6385fd0d-256d-4400-9319-afa8170024db",
    scryfall_id = "50c33f18-0a5c-4e46-ab0d-6e450915594f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Culling the Weak",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_CREATURE)],
    ),],
    abilities = &[spell!(&[Effect::mana(ManaColor::Black, 4)])],
);
