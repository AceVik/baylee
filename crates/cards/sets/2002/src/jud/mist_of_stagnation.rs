//! Mist of Stagnation — {3}{U}{U} — Enchantment
//! Oracle: Permanents don't untap during their controllers' untap steps.
//! Oracle: At the beginning of each player's upkeep, that player chooses a permanent for each card in their graveyard, then untaps those permanents.
//! Set: JUD #48 — Judgment | Scryfall ID: 76d03121-9515-4101-9d60-e01225533f44 | Oracle ID: e78673c4-a969-443f-b1a4-0eb37b8111af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIST_OF_STAGNATION,
    oracle_id = "e78673c4-a969-443f-b1a4-0eb37b8111af",
    scryfall_id = "76d03121-9515-4101-9d60-e01225533f44",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mist of Stagnation",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
