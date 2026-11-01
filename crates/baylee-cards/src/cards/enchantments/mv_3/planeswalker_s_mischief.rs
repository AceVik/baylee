//! Planeswalker's Mischief — {2}{U} — Enchantment
//! Oracle: {3}{U}: Target opponent reveals a card at random from their hand. If it's an instant or sorcery card, exile it. You may cast it without paying its mana cost for as long as it remains exiled. At the beginning of the next end step, if you haven't cast it, return it to its owner's hand. Activate only as a sorcery.
//! Set: PLS #29 — Planeshift | Scryfall ID: 79aa232c-3f16-4c68-99dc-09a7aeef477b | Oracle ID: 78e87805-19e3-415b-ad0b-3275183d7297
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANESWALKER_S_MISCHIEF,
    oracle_id = "78e87805-19e3-415b-ad0b-3275183d7297",
    scryfall_id = "79aa232c-3f16-4c68-99dc-09a7aeef477b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Planeswalker's Mischief",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
