//! Desert — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: This land deals 1 damage to target attacking creature. Activate only during the end of combat step.
//! Set: AFC #233 — Forgotten Realms Commander | Scryfall ID: c74e13eb-6f82-4db1-9d0d-8310f48d9f6d | Oracle ID: 195107ad-879d-4b02-a44a-a3ba70fedf88
// PARTIAL — {T}: Add {C} is written; the damage ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESERT,
    oracle_id = "195107ad-879d-4b02-a44a-a3ba70fedf88",
    scryfall_id = "c74e13eb-6f82-4db1-9d0d-8310f48d9f6d",
    faces = &[face!(
        name = "Desert",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Partial(
        "the damage ability may be activated only during the end of combat \
         step, and ActivationTiming has only InstantSpeed and SorcerySpeed"
    ),
    // NOT SUPPORTED: "{T}: This land deals 1 damage to target attacking
    // creature. Activate only during the end of combat step." — the effect
    // itself is sayable (`Effect::DealDamage` to `Filter::ATTACKING_CREATURE`),
    // but the printed window is not, and an ability offered at every priority
    // is a Desert that burns attackers whenever it likes. So it comes off the
    // card instead of shipping stronger than the card.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
