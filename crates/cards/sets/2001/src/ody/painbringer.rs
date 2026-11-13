//! Painbringer — {2}{B}{B} — Creature — Human Minion
//! Oracle: {T}, Exile any number of cards from your graveyard: Target creature gets -X/-X until end of turn, where X is the number of cards exiled this way.
//! Set: ODY #155 — Odyssey | Scryfall ID: 3f993815-af2f-4fa0-8848-979fc8c72d9a | Oracle ID: 6ca49880-4e12-494b-b14e-e49385d0f7f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PAINBRINGER,
    oracle_id = "6ca49880-4e12-494b-b14e-e49385d0f7f5",
    scryfall_id = "3f993815-af2f-4fa0-8848-979fc8c72d9a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Painbringer",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MINION],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
