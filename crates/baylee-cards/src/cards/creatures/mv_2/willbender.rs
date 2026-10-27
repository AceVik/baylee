//! Willbender — {1}{U} — Creature — Human Wizard
//! Oracle: Morph {1}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, change the target of target spell or ability with a single target.
//! Set: C19 #102 — Commander 2019 | Scryfall ID: 6e6a3551-250c-4563-8c30-6d8f49e6429f | Oracle ID: 0aae277e-e58e-4115-b5fd-0459451e17ec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WILLBENDER,
    oracle_id = "0aae277e-e58e-4115-b5fd-0459451e17ec",
    scryfall_id = "6e6a3551-250c-4563-8c30-6d8f49e6429f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Willbender",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
