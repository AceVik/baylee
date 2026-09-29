//! Crusade — {W}{W} — Enchantment
//! Oracle: White creatures get +1/+1.
//! Set: DDF #27 — Duel Decks: Elspeth vs. Tezzeret | Scryfall ID: b99452c0-5d1c-4a73-90b6-0ec3ac0af893 | Oracle ID: 4692740f-be90-459f-8d90-c4ae71771595
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRUSADE,
    oracle_id = "4692740f-be90-459f-8d90-c4ae71771595",
    scryfall_id = "b99452c0-5d1c-4a73-90b6-0ec3ac0af893",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Crusade",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
