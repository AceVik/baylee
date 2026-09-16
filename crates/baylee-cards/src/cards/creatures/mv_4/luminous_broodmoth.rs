//! Luminous Broodmoth — {2}{W}{W} — Creature — Insect
//! Oracle: Flying
//! Oracle: Whenever a creature you control without flying dies, return it to the battlefield under its owner's control with a flying counter on it.
//! Set: IKO #21 — Ikoria: Lair of Behemoths | Scryfall ID: bb65df55-d6a6-4a57-a903-e5eb17637982 | Oracle ID: 28c7c816-07e7-42fb-923c-bf149ba28b38
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LUMINOUS_BROODMOTH,
    oracle_id = "28c7c816-07e7-42fb-923c-bf149ba28b38",
    scryfall_id = "bb65df55-d6a6-4a57-a903-e5eb17637982",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Luminous Broodmoth",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
