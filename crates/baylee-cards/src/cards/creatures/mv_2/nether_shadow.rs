//! Nether Shadow — {B}{B} — Creature — Spirit
//! Oracle: Haste
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard with three or more creature cards above it, you may put this card onto the battlefield.
//! Set: ME1 #77 — Masters Edition | Scryfall ID: 11988c46-6d0c-46a8-85aa-8c6da72bfe30 | Oracle ID: c358b9e2-524c-434b-b3fa-74d2aa6d1df7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NETHER_SHADOW,
    oracle_id = "c358b9e2-524c-434b-b3fa-74d2aa6d1df7",
    scryfall_id = "11988c46-6d0c-46a8-85aa-8c6da72bfe30",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nether Shadow",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
