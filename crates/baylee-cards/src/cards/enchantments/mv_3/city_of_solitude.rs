//! City of Solitude — {2}{G} — Enchantment
//! Oracle: Players can cast spells and activate abilities only during their own turns.
//! Set: VIS #102 — Visions | Scryfall ID: be499b81-bb2d-4f1d-9deb-c8bfcdca8e13 | Oracle ID: 671458ac-7441-4332-9e59-a42ec8b62f31
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_OF_SOLITUDE,
    oracle_id = "671458ac-7441-4332-9e59-a42ec8b62f31",
    scryfall_id = "be499b81-bb2d-4f1d-9deb-c8bfcdca8e13",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "City of Solitude",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
