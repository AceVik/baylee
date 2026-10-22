//! Rolling Thunder — {X}{R}{R} — Sorcery
//! Oracle: Rolling Thunder deals X damage divided as you choose among any number of targets.
//! Set: GNT #40 — Game Night | Scryfall ID: 0df50703-5655-4bc4-adc9-d719c2da3bfd | Oracle ID: 9c47888b-28a5-4c43-9ee4-a9059e3c367d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROLLING_THUNDER,
    oracle_id = "9c47888b-28a5-4c43-9ee4-a9059e3c367d",
    scryfall_id = "0df50703-5655-4bc4-adc9-d719c2da3bfd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rolling Thunder",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::XDoubleFrom(u32::MAX),
        }],
        targets = Some(TargetReq::up_to(TargetSpec::AnyTarget, u8::MAX)),
    )],
);
