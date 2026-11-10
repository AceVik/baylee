//! True Believer — {W}{W} — Creature — Human Cleric
//! Oracle: You have shroud. (You can't be the target of spells or abilities.)
//! Set: 10E #53 — Tenth Edition | Scryfall ID: 85a9c5c4-47d1-4b87-8cbd-2212ee6c5887 | Oracle ID: 5a0cd4dd-2212-4408-819e-0cb7c150418b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRUE_BELIEVER,
    oracle_id = "5a0cd4dd-2212-4408-819e-0cb7c150418b",
    scryfall_id = "85a9c5c4-47d1-4b87-8cbd-2212ee6c5887",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "True Believer",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
