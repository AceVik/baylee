//! Sacrifice — {B} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Add an amount of {B} equal to the sacrificed creature's mana value.
//! Set: SUM #126 — Summer Magic / Edgar | Scryfall ID: 652bd781-1a48-42c1-9aff-f9050a9285d6 | Oracle ID: 068b3692-411b-44d4-a7e9-005262760cfc

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SACRIFICE,
    oracle_id = "068b3692-411b-44d4-a7e9-005262760cfc",
    scryfall_id = "652bd781-1a48-42c1-9aff-f9050a9285d6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Sacrifice",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::mana_dynamic(
        ManaColor::Black,
        Amount::SacrificedManaValue
    )]),],
);
