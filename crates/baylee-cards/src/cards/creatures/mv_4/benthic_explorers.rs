//! Benthic Explorers — {3}{U} — Creature — Merfolk Scout
//! Oracle: {T}, Untap a tapped land an opponent controls: Add one mana of any type that land could produce.
//! Set: ME3 #29 — Masters Edition III | Scryfall ID: d7eca8c7-cc51-44d6-a529-ccf128136c07 | Oracle ID: bc0518fc-d34b-474a-92c5-2cb013828396
// NOT SUPPORTED: untapping a tapped land an opponent controls as part of the activation cost and restricting the mana to a type that land could produce are not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BENTHIC_EXPLORERS,
    oracle_id = "bc0518fc-d34b-474a-92c5-2cb013828396",
    scryfall_id = "d7eca8c7-cc51-44d6-a529-ccf128136c07",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "{T} adds one mana of any color; untapping a tapped land an opponent controls as part of the activation cost and restricting the mana to a type that land could produce are not supported",
    ),
    faces = &[face!(
        name = "Benthic Explorers",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SCOUT],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[mana_ability!(&[Effect::mana_of_any_color()])],
);
