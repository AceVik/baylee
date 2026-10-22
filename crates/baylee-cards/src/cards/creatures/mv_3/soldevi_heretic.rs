//! Soldevi Heretic — {2}{U} — Creature — Human Cleric
//! Oracle: {W}, {T}: Prevent the next 2 damage that would be dealt to target creature this turn. Target opponent may draw a card.
//! Set: ALL #33a — Alliances | Scryfall ID: d46accc8-b926-4443-bc12-dfd5870b2d2e | Oracle ID: eed353a6-757d-47f6-a118-b951ccf95600

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
        "The {W}, {T} ability prevents the next 2 damage that would be dealt to target creature this turn, without targeting an opponent or offering that opponent a card draw."
    ),
    abilities = &[activated!(
        cost!("{W}", TapSelf),
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::Fixed(2),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
