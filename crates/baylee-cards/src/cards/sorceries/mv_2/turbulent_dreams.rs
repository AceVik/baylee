//! Turbulent Dreams — {U}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Return X target nonland permanents to their owners' hands.
//! Set: TOR #49 — Torment | Scryfall ID: d879226b-8ee3-4a48-b03a-d183594dc586 | Oracle ID: 83134570-debe-4bc0-9656-2a16801a3592
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TURBULENT_DREAMS,
    oracle_id = "83134570-debe-4bc0-9656-2a16801a3592",
    scryfall_id = "d879226b-8ee3-4a48-b03a-d183594dc586",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Turbulent Dreams",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
