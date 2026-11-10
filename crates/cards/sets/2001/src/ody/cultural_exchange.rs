//! Cultural Exchange — {4}{U}{U} — Sorcery
//! Oracle: Choose any number of creatures target player controls. Choose the same number of creatures another target player controls. Those players exchange control of those creatures. (This effect lasts indefinitely.)
//! Set: ODY #79 — Odyssey | Scryfall ID: d4f38d07-829c-4311-a61b-2785cf2266ef | Oracle ID: c85ba59f-2851-4c75-86db-da798334850c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CULTURAL_EXCHANGE,
    oracle_id = "c85ba59f-2851-4c75-86db-da798334850c",
    scryfall_id = "d4f38d07-829c-4311-a61b-2785cf2266ef",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cultural Exchange",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
