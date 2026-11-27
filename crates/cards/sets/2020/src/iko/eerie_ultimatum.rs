//! Eerie Ultimatum — {W}{W}{B}{B}{B}{G}{G} — Sorcery
//! Oracle: Return any number of permanent cards with different names from your graveyard to the battlefield.
//! Set: IKO #184 — Ikoria: Lair of Behemoths | Scryfall ID: 3a9fb2db-228f-4d48-acdf-6330baf356c7 | Oracle ID: 5674f6ae-ed5d-441e-a534-b5dd415165fd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EERIE_ULTIMATUM,
    oracle_id = "5674f6ae-ed5d-441e-a534-b5dd415165fd",
    scryfall_id = "3a9fb2db-228f-4d48-acdf-6330baf356c7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
    faces = &[face!(
        name = "Eerie Ultimatum",
        mana_cost = mana!("{W}{W}{B}{B}{B}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
