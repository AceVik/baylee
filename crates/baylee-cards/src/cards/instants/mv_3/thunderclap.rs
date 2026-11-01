//! Thunderclap — {2}{R} — Instant
//! Oracle: You may sacrifice a Mountain rather than pay this spell's mana cost.
//! Oracle: Thunderclap deals 3 damage to target creature.
//! Set: MMQ #219 — Mercadian Masques | Scryfall ID: b3f8c5ee-2179-4c05-adc9-0b66d02b59ad | Oracle ID: 02f5ceb4-8b9c-4107-972b-6eec365c4a6c
// NOT SUPPORTED: Alternate cost of sacrificing a Mountain is not supported; spell effect works when cast for {2}{R}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THUNDERCLAP,
    oracle_id = "02f5ceb4-8b9c-4107-972b-6eec365c4a6c",
    scryfall_id = "b3f8c5ee-2179-4c05-adc9-0b66d02b59ad",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("alternate sacrifice-a-Mountain cost not supported"),
    faces = &[face!(
        name = "Thunderclap",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
