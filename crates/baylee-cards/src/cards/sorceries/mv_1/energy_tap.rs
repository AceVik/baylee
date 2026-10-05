//! Energy Tap — {U} — Sorcery
//! Oracle: Tap target untapped creature you control. If you do, add an amount of {C} equal to that creature's mana value.
//! Set: 4ED #69 — Fourth Edition | Scryfall ID: 9f67692d-9df6-4841-a36b-26c28110b63b | Oracle ID: 106ae855-574b-411d-8689-cbb9eeb4f602
// IMPLEMENTED — taps the target and adds {C} equal to its mana value; a creature tapped in response is illegal and fizzles the spell (CR 608.2b), which is the printed "If you do".

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_TAP,
    oracle_id = "106ae855-574b-411d-8689-cbb9eeb4f602",
    scryfall_id = "9f67692d-9df6-4841-a36b-26c28110b63b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Energy Tap",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            Effect::mana_dynamic(ManaColor::Colorless, Amount::TargetCmc),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &f!(untapped your CREATURE)
        )))
    )],
);
