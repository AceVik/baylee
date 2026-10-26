//! Flameshot — {3}{R} — Sorcery
//! Oracle: You may discard a Mountain card rather than pay this spell's mana cost.
//! Oracle: Flameshot deals 3 damage divided as you choose among one, two, or three target creatures.
//! Set: PCY #90 — Prophecy | Scryfall ID: 3f7b61a8-a1ff-4e2a-bc24-8990c61a5e5b | Oracle ID: 26640648-6400-4b80-a78b-f519ab9cd09e
// NOT SUPPORTED: deals 3 damage divided as you choose among one, two, or three target creatures for {3}{R}; the alternative cost of discarding a Mountain card is not yet modeled
// PARTIAL: deals 3 damage divided as you choose among one, two, or three target creatures for {3}{R}; the alternative cost of discarding a Mountain card is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAMESHOT,
    oracle_id = "26640648-6400-4b80-a78b-f519ab9cd09e",
    scryfall_id = "3f7b61a8-a1ff-4e2a-bc24-8990c61a5e5b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "deals 3 damage divided as you choose among one, two, or three target creatures for {3}{R}; the alternative cost of discarding a Mountain card is not yet modeled",
    ),
    faces = &[face!(
        name = "Flameshot",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::Fixed(3),
        }],
        targets = Some(TargetReq {
            spec: TargetSpec::Object(&Filter::CREATURE),
            min: 1,
            max: 3,
            count_is_x: false,
            distinct_mana_values: false,
            chooser: TargetChooser::Controller,
        }),
    ),],
);
