//! Purgatory — {2}{W}{B} — Enchantment
//! Oracle: Whenever a nontoken creature is put into your graveyard from the battlefield, exile that card.
//! Oracle: At the beginning of your upkeep, you may pay {4} and 2 life. If you do, return a card exiled with this enchantment to the battlefield.
//! Set: MIR #275 — Mirage | Scryfall ID: 9ed5e58b-46f3-437c-89ac-24235a65bd1f | Oracle ID: 2fb8e100-58a1-4dc6-bc38-ad7f115e3e46
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PURGATORY,
    oracle_id = "2fb8e100-58a1-4dc6-bc38-ad7f115e3e46",
    scryfall_id = "9ed5e58b-46f3-437c-89ac-24235a65bd1f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Purgatory",
        mana_cost = mana!("{2}{W}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
