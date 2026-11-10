//! Hypnox — {8}{B}{B}{B} — Creature — Nightmare Horror
//! Oracle: Flying
//! Oracle: When this creature enters, if you cast it from your hand, exile all cards from target opponent's hand.
//! Oracle: When this creature leaves the battlefield, return the exiled cards to their owner's hand.
//! Set: TOR #64 — Torment | Scryfall ID: 0e0e53a1-11d5-4975-b698-8b5d72d241f0 | Oracle ID: b850015f-527f-4e5a-87a6-46a0814bdb70
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HYPNOX,
    oracle_id = "b850015f-527f-4e5a-87a6-46a0814bdb70",
    scryfall_id = "0e0e53a1-11d5-4975-b698-8b5d72d241f0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hypnox",
        mana_cost = mana!("{8}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORROR],
        power = Some(8),
        toughness = Some(8),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
