//! War Tax — {2}{U} — Enchantment
//! Oracle: {X}{U}: This turn, creatures can't attack unless their controller pays {X} for each attacking creature they control.
//! Set: MMQ #113 — Mercadian Masques | Scryfall ID: e7c15159-8466-43e5-9dc5-a8cc94619931 | Oracle ID: fd5ff6ad-f3f1-470f-adef-fb4ce6536365
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAR_TAX,
    oracle_id = "fd5ff6ad-f3f1-470f-adef-fb4ce6536365",
    scryfall_id = "e7c15159-8466-43e5-9dc5-a8cc94619931",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "War Tax",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
