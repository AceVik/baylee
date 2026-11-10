//! War Cadence — {2}{R} — Enchantment
//! Oracle: {X}{R}: This turn, creatures can't block unless their controller pays {X} for each blocking creature they control.
//! Set: C13 #128 — Commander 2013 | Scryfall ID: 359434c4-7123-435b-890b-4e88f5bf2f49 | Oracle ID: 49d0fdd6-cc8f-4fe1-a6bd-4321dac18404
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAR_CADENCE,
    oracle_id = "49d0fdd6-cc8f-4fe1-a6bd-4321dac18404",
    scryfall_id = "359434c4-7123-435b-890b-4e88f5bf2f49",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "War Cadence",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
