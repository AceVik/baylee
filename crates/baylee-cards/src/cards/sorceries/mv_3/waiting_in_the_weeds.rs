//! Waiting in the Weeds — {1}{G}{G} — Sorcery
//! Oracle: Each player creates a 1/1 green Cat creature token for each untapped Forest they control.
//! Set: 6ED #266 — Classic Sixth Edition | Scryfall ID: 7e2d5a77-6ee8-463d-9e6a-fdb1cc6d70e6 | Oracle ID: 5a61d51e-1a3f-49f6-9a94-a708dd6f69df
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAITING_IN_THE_WEEDS,
    oracle_id = "5a61d51e-1a3f-49f6-9a94-a708dd6f69df",
    scryfall_id = "7e2d5a77-6ee8-463d-9e6a-fdb1cc6d70e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Waiting in the Weeds",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
