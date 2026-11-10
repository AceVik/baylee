//! Chain of Acid — {3}{G} — Sorcery
//! Oracle: Destroy target noncreature permanent. Then that permanent's controller may copy this spell and may choose a new target for that copy.
//! Set: ONS #252 — Onslaught | Scryfall ID: 1d47ddca-a363-4ab7-b7f2-d0e0043c9916 | Oracle ID: 0b1a27bd-bb98-44f8-8357-666fabfeabf0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_OF_ACID,
    oracle_id = "0b1a27bd-bb98-44f8-8357-666fabfeabf0",
    scryfall_id = "1d47ddca-a363-4ab7-b7f2-d0e0043c9916",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Chain of Acid",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
