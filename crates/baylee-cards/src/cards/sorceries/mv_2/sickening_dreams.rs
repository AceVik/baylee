//! Sickening Dreams — {1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Sickening Dreams deals X damage to each creature and each player.
//! Set: TOR #83 — Torment | Scryfall ID: 9396ac77-9f53-46bd-b126-02441a0f5594 | Oracle ID: 728d2e30-2f36-4054-81db-29b01e376a82
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SICKENING_DREAMS,
    oracle_id = "728d2e30-2f36-4054-81db-29b01e376a82",
    scryfall_id = "9396ac77-9f53-46bd-b126-02441a0f5594",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Sickening Dreams",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
