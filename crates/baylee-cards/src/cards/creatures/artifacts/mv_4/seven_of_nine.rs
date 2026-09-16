//! Seven of Nine — {2}{U}{B} — Legendary Artifact Creature — Human Scientist
//! Oracle: Whenever you face a dilemma, draw a card. (You face a dilemma as you choose one or more modes for a spell or ability.)
//! Set: TRK #252 — Star Trek | Scryfall ID: 6204cfdb-6fb4-4030-96b6-9306c8266bcf | Oracle ID: 3c037865-d38c-48da-aee9-308975e12638
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEVEN_OF_NINE,
    oracle_id = "3c037865-d38c-48da-aee9-308975e12638",
    scryfall_id = "6204cfdb-6fb4-4030-96b6-9306c8266bcf",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Seven of Nine",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SCIENTIST],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
