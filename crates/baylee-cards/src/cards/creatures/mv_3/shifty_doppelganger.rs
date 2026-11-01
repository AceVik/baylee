//! Shifty Doppelganger — {2}{U} — Creature — Shapeshifter
//! Oracle: {3}{U}, Exile this creature: You may put a creature card from your hand onto the battlefield. If you do, that creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature. If you do, return this card to the battlefield.
//! Set: ODY #101 — Odyssey | Scryfall ID: 538ffe1e-c312-4797-99dd-9bf5f896bdc3 | Oracle ID: b9be00a5-448f-4c90-b9ca-b30bcdfe95f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIFTY_DOPPELGANGER,
    oracle_id = "b9be00a5-448f-4c90-b9ca-b30bcdfe95f2",
    scryfall_id = "538ffe1e-c312-4797-99dd-9bf5f896bdc3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shifty Doppelganger",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
