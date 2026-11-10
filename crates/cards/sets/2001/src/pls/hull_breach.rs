//! Hull Breach — {R}{G} — Sorcery
//! Oracle: Choose one —
//! Oracle: • Destroy target artifact.
//! Oracle: • Destroy target enchantment.
//! Oracle: • Destroy target artifact and target enchantment.
//! Set: MSC #185 — Marvel Super Heroes Commander | Scryfall ID: 3ff49b0b-322b-4b98-8604-d6716f3bec39 | Oracle ID: 2da232d8-580f-4116-b977-2c59cd21b5a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HULL_BREACH,
    oracle_id = "2da232d8-580f-4116-b977-2c59cd21b5a4",
    scryfall_id = "3ff49b0b-322b-4b98-8604-d6716f3bec39",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Hull Breach",
        mana_cost = mana!("{R}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
