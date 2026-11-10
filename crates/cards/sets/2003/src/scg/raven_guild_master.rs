//! Raven Guild Master — {1}{U}{U} — Creature — Human Wizard Mutant
//! Oracle: Whenever this creature deals combat damage to a player, that player exiles the top ten cards of their library.
//! Oracle: Morph {2}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: SCG #47 — Scourge | Scryfall ID: 9843f847-6a8f-4042-86b6-f7fe5a47cc57 | Oracle ID: 1dfaa773-7cd5-4489-9790-661cef4ed6f3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAVEN_GUILD_MASTER,
    oracle_id = "1dfaa773-7cd5-4489-9790-661cef4ed6f3",
    scryfall_id = "9843f847-6a8f-4042-86b6-f7fe5a47cc57",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Raven Guild Master",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::MUTANT
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
