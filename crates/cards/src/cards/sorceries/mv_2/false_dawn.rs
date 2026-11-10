//! False Dawn — {1}{W} — Sorcery
//! Oracle: Until end of turn, spells and abilities you control that would add colored mana instead add that much white mana. Until end of turn, you may spend white mana as though it were mana of any color.
//! Oracle: Draw a card.
//! Set: APC #10 — Apocalypse | Scryfall ID: 1695e0ba-005a-4652-aea7-e1d1f9ff5d66 | Oracle ID: efd9cc25-f445-4de0-9a69-adab9f8878f4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALSE_DAWN,
    oracle_id = "efd9cc25-f445-4de0-9a69-adab9f8878f4",
    scryfall_id = "1695e0ba-005a-4652-aea7-e1d1f9ff5d66",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "False Dawn",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
