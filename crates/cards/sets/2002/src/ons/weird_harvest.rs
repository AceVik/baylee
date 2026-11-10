//! Weird Harvest — {X}{G}{G} — Sorcery
//! Oracle: Each player may search their library for up to X creature cards, reveal those cards, put them into their hand, then shuffle.
//! Set: 9ED #282 — Ninth Edition | Scryfall ID: 89bd2c25-049a-4df2-bf8f-198d1b41b01e | Oracle ID: 76587fa5-29eb-480f-a865-480cd39b1da7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WEIRD_HARVEST,
    oracle_id = "76587fa5-29eb-480f-a865-480cd39b1da7",
    scryfall_id = "89bd2c25-049a-4df2-bf8f-198d1b41b01e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Weird Harvest",
        mana_cost = mana!("{X}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
