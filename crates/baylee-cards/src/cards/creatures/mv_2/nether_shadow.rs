//! Nether Shadow — {B}{B} — Creature — Spirit
//! Oracle: Haste
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard with three or more creature cards above it, you may put this card onto the battlefield.
//! Set: ME1 #77 — Masters Edition | Scryfall ID: 11988c46-6d0c-46a8-85aa-8c6da72bfe30 | Oracle ID: c358b9e2-524c-434b-b3fa-74d2aa6d1df7
// PARTIAL — returning from the graveyard by the creature cards above it is not
// in the engine; it has haste.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NETHER_SHADOW,
    oracle_id = "c358b9e2-524c-434b-b3fa-74d2aa6d1df7",
    scryfall_id = "11988c46-6d0c-46a8-85aa-8c6da72bfe30",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Partial(
        "returning from the graveyard by the creature cards above it is not in the engine; it has haste"
    ),
    faces = &[face!(
        name = "Nether Shadow",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        // NOT SUPPORTED: At the beginning of your upkeep, if this card is in your
        // graveyard with three or more creature cards above it, you may put this card onto
        // the battlefield.
    ],
);
