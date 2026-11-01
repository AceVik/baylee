//! Noble Benefactor — {2}{U} — Creature — Human Cleric
//! Oracle: When this creature dies, each player may search their library for a card and put that card into their hand. Then each player who searched their library this way shuffles.
//! Set: WTH #44 — Weatherlight | Scryfall ID: bd221f30-1773-4e05-a40f-022a9306ef89 | Oracle ID: 78c1d6fe-38e4-40c4-9252-87ff272247ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOBLE_BENEFACTOR,
    oracle_id = "78c1d6fe-38e4-40c4-9252-87ff272247ad",
    scryfall_id = "bd221f30-1773-4e05-a40f-022a9306ef89",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Noble Benefactor",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
