//! Cadaverous Bloom — {3}{B}{G} — Enchantment
//! Oracle: Exile a card from your hand: Add {B}{B} or {G}{G}.
//! Set: MIR #258 — Mirage | Scryfall ID: c9bef70b-61c7-4df5-b4df-09cd6ab2015c | Oracle ID: fbb0f73b-5e30-4632-99c1-e49582e41f8d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CADAVEROUS_BLOOM,
    oracle_id = "fbb0f73b-5e30-4632-99c1-e49582e41f8d",
    scryfall_id = "c9bef70b-61c7-4df5-b4df-09cd6ab2015c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Cadaverous Bloom",
        mana_cost = mana!("{3}{B}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
