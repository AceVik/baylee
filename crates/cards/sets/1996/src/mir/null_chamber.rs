//! Null Chamber — {3}{W} — World Enchantment
//! Oracle: As this enchantment enters, you and an opponent each choose a card name other than a basic land card name.
//! Oracle: Spells with the chosen names can't be cast and lands with the chosen names can't be played.
//! Set: MIR #31 — Mirage | Scryfall ID: 814f8976-6612-438f-a04a-8edb63edb1e7 | Oracle ID: cb75aca6-dfc7-460d-b90c-80b7988f39a8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NULL_CHAMBER,
    oracle_id = "cb75aca6-dfc7-460d-b90c-80b7988f39a8",
    scryfall_id = "814f8976-6612-438f-a04a-8edb63edb1e7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Null Chamber",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
