//! Dream Halls — {3}{U}{U} — Enchantment
//! Oracle: Rather than pay the mana cost for a spell, its controller may discard a card that shares a color with that spell.
//! Set: TPR #46 — Tempest Remastered | Scryfall ID: 2c6606d1-619f-4cc9-8d5b-771c4b4d9615 | Oracle ID: d901e17f-a5f4-49d9-8a6d-ab771949af24
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DREAM_HALLS,
    oracle_id = "d901e17f-a5f4-49d9-8a6d-ab771949af24",
    scryfall_id = "2c6606d1-619f-4cc9-8d5b-771c4b4d9615",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dream Halls",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
