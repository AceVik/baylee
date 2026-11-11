//! Madame Null, Power Broker — {2}{B} — Legendary Creature — Demon Advisor
//! Oracle: Deathtouch
//! Oracle: Whenever another creature you control enters, you may pay life equal to its power. If you do, put that many +1/+1 counters on it.
//! Set: TMT #66 — Teenage Mutant Ninja Turtles | Scryfall ID: 4e712d5c-d6fe-40bb-b8e5-5f50c6ea8d4f | Oracle ID: 73413bd5-8c40-4d45-8f05-c20c39ee4017
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MADAME_NULL_POWER_BROKER,
    oracle_id = "73413bd5-8c40-4d45-8f05-c20c39ee4017",
    scryfall_id = "4e712d5c-d6fe-40bb-b8e5-5f50c6ea8d4f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Madame Null, Power Broker",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DEMON, subtypes::creature::ADVISOR],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
