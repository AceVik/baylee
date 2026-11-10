//! Kilnmouth Dragon — {5}{R}{R} — Creature — Dragon
//! Oracle: Amplify 3 (As this creature enters, put three +1/+1 counters on it for each Dragon card you reveal in your hand.)
//! Oracle: Flying
//! Oracle: {T}: This creature deals damage equal to the number of +1/+1 counters on it to any target.
//! Set: DDG #59 — Duel Decks: Knights vs. Dragons | Scryfall ID: 67bbc1c3-8806-49d4-9111-a2334c66e5ec | Oracle ID: 5cb73cd3-2cd2-45e2-a593-a498a73b2cf5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KILNMOUTH_DRAGON,
    oracle_id = "5cb73cd3-2cd2-45e2-a593-a498a73b2cf5",
    scryfall_id = "67bbc1c3-8806-49d4-9111-a2334c66e5ec",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kilnmouth Dragon",
        mana_cost = mana!("{5}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
