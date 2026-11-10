//! Mistform Mutant — {4}{U}{U} — Creature — Illusion Mutant
//! Oracle: {1}{U}: Choose a creature type other than Wall. Target creature becomes that type until end of turn.
//! Set: ONS #95 — Onslaught | Scryfall ID: a25b2697-5d7f-490a-8474-c775096e681e | Oracle ID: 0f4350e2-48c7-4d2b-9384-4864232756ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_MUTANT,
    oracle_id = "0f4350e2-48c7-4d2b-9384-4864232756ad",
    scryfall_id = "a25b2697-5d7f-490a-8474-c775096e681e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Mutant",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION, subtypes::creature::MUTANT],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
