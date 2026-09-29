//! Shanodin Dryads — {G} — Creature — Nymph Dryad
//! Oracle: Forestwalk (This creature can't be blocked as long as defending player controls a Forest.)
//! Set: 7ED #269 — Seventh Edition | Scryfall ID: 90e8ff87-22e8-4c04-84bc-f0ea2c12a86c | Oracle ID: 998484cc-fefc-4da5-9987-5d6e89599c34
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHANODIN_DRYADS,
    oracle_id = "998484cc-fefc-4da5-9987-5d6e89599c34",
    scryfall_id = "90e8ff87-22e8-4c04-84bc-f0ea2c12a86c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Shanodin Dryads",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NYMPH, subtypes::creature::DRYAD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
