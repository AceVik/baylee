//! Penance — {2}{W} — Enchantment
//! Oracle: Put a card from your hand on top of your library: The next time a black or red source of your choice would deal damage this turn, prevent that damage.
//! Set: EXO #15 — Exodus | Scryfall ID: 1f3db848-8394-43bd-a236-264641033a6d | Oracle ID: 7818d6e0-74d8-46ae-94f6-d2a349c9507b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PENANCE,
    oracle_id = "7818d6e0-74d8-46ae-94f6-d2a349c9507b",
    scryfall_id = "1f3db848-8394-43bd-a236-264641033a6d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Penance",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
