//! No Rest for the Wicked — {1}{B} — Enchantment
//! Oracle: Sacrifice this enchantment: Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn.
//! Set: USG #142 — Urza's Saga | Scryfall ID: d9f9b2b0-65aa-42a1-b9f8-563b194ca5b1 | Oracle ID: d03fd358-ff3f-45ab-9601-07dbb535cb51
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NO_REST_FOR_THE_WICKED,
    oracle_id = "d03fd358-ff3f-45ab-9601-07dbb535cb51",
    scryfall_id = "d9f9b2b0-65aa-42a1-b9f8-563b194ca5b1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "No Rest for the Wicked",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
