//! Cabal Inquisitor — {1}{B} — Creature — Human Minion
//! Oracle: Threshold — {1}{B}, {T}, Exile two cards from your graveyard: Target player discards a card. Activate only as a sorcery and only if there are seven or more cards in your graveyard.
//! Set: ODY #119 — Odyssey | Scryfall ID: c0530e09-5206-460e-abd2-e928eca294a5 | Oracle ID: 5bebf38e-bff6-470b-a2b8-cb0a87a7372f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CABAL_INQUISITOR,
    oracle_id = "5bebf38e-bff6-470b-a2b8-cb0a87a7372f",
    scryfall_id = "c0530e09-5206-460e-abd2-e928eca294a5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Inquisitor",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MINION],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
