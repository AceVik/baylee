//! Soldevi Heretic — {2}{U} — Creature — Human Cleric
//! Oracle: {W}, {T}: Prevent the next 2 damage that would be dealt to target creature this turn. Target opponent may draw a card.
//! Set: ALL #33a — Alliances | Scryfall ID: d46accc8-b926-4443-bc12-dfd5870b2d2e | Oracle ID: eed353a6-757d-47f6-a118-b951ccf95600

// PARTIAL — a vanilla 2/2: the {W}, {T} ability is not played.
// NOT SUPPORTED: "{W}, {T}: Prevent the next 2 damage that would be dealt to target creature this turn. Target opponent may draw a card." The ability has no second target for an opponent, and without "Target opponent may draw a card" (the price of the shield) it would play stronger than printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLDEVI_HERETIC,
    oracle_id = "eed353a6-757d-47f6-a118-b951ccf95600",
    scryfall_id = "d46accc8-b926-4443-bc12-dfd5870b2d2e",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Soldevi Heretic",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "The {W}, {T} ability is not played: its drawback \"Target opponent may draw a card\" cannot be said (no second target for an opponent), and the damage shield without it would play stronger than printed; Soldevi Heretic enters as a 2/2 with no abilities."
    ),
);
