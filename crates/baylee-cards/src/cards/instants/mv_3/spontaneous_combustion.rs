//! Spontaneous Combustion — {1}{B}{R} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Spontaneous Combustion deals 3 damage to each creature.
//! Set: CNS #194 — Conspiracy | Scryfall ID: 55d50177-736a-44d6-a2a3-f6892d7037b3 | Oracle ID: 0f81064d-01cd-4a61-8aea-2544e4d14b70

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPONTANEOUS_COMBUSTION,
    oracle_id = "0f81064d-01cd-4a61-8aea-2544e4d14b70",
    scryfall_id = "55d50177-736a-44d6-a2a3-f6892d7037b3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spontaneous Combustion",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_CREATURE)],
    ),],
    abilities = &[spell!(&[Effect::DealDamageEach {
        amount: Amount::Fixed(3),
        filter: &Filter::CREATURE,
    }])],
);
