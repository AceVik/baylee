//! Astral Slide — {2}{W} — Enchantment
//! Oracle: Whenever a player cycles a card, you may exile target creature. If you do, return that card to the battlefield under its owner's control at the beginning of the next end step.
//! Set: VMA #13 — Vintage Masters | Scryfall ID: ba089228-135a-421b-b932-26b86e555b78 | Oracle ID: 70f14b10-8129-4249-a267-8b4be5d9befa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASTRAL_SLIDE,
    oracle_id = "70f14b10-8129-4249-a267-8b4be5d9befa",
    scryfall_id = "ba089228-135a-421b-b932-26b86e555b78",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Astral Slide",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
