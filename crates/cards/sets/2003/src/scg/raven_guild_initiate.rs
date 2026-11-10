//! Raven Guild Initiate — {2}{U} — Creature — Human Wizard
//! Oracle: Morph—Return a Bird you control to its owner's hand. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: SCG #46 — Scourge | Scryfall ID: c1e11f70-06c3-4dc5-aafe-82d65080085e | Oracle ID: 67a1529f-aa37-4df8-97e3-5b1d8c03a76f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAVEN_GUILD_INITIATE,
    oracle_id = "67a1529f-aa37-4df8-97e3-5b1d8c03a76f",
    scryfall_id = "c1e11f70-06c3-4dc5-aafe-82d65080085e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Raven Guild Initiate",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
