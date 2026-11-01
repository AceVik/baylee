//! Spurnmage Advocate — {W} — Creature — Human Nomad
//! Oracle: {T}: Return two target cards from an opponent's graveyard to their hand. Destroy target attacking creature.
//! Set: CMD #33 — Commander 2011 | Scryfall ID: 9c014b89-895c-432f-94f5-cda2dfac57e4 | Oracle ID: 0477b923-9f32-4e7f-ac08-c715cca819a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPURNMAGE_ADVOCATE,
    oracle_id = "0477b923-9f32-4e7f-ac08-c715cca819a4",
    scryfall_id = "9c014b89-895c-432f-94f5-cda2dfac57e4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Spurnmage Advocate",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
