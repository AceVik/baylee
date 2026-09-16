//! Endurance — {1}{G}{G} — Creature — Elemental Incarnation
//! Oracle: Flash
//! Oracle: Reach
//! Oracle: When this creature enters, up to one target player puts all the cards from their graveyard on the bottom of their library in a random order.
//! Oracle: Evoke—Exile a green card from your hand.
//! Set: ECC #51 — Lorwyn Eclipsed Commander | Scryfall ID: b770471c-1bf7-4179-8418-dcd790ca5405 | Oracle ID: c85d824b-c190-4d04-ab99-918ad0e6516c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENDURANCE,
    oracle_id = "c85d824b-c190-4d04-ab99-918ad0e6516c",
    scryfall_id = "b770471c-1bf7-4179-8418-dcd790ca5405",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Endurance",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELEMENTAL,
            subtypes::creature::INCARNATION
        ],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
