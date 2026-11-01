//! Choking Tethers — {3}{U} — Instant
//! Oracle: Tap up to four target creatures.
//! Oracle: Cycling {1}{U} ({1}{U}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may tap target creature.
//! Set: MH1 #44 — Modern Horizons | Scryfall ID: eb0d0c81-8698-41db-a687-47af028e910e | Oracle ID: a60348fa-b182-4667-9e38-15bdf3511f82
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHOKING_TETHERS,
    oracle_id = "a60348fa-b182-4667-9e38-15bdf3511f82",
    scryfall_id = "eb0d0c81-8698-41db-a687-47af028e910e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Choking Tethers",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
