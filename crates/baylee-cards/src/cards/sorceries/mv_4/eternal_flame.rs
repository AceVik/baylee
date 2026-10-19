//! Eternal Flame — {2}{R}{R} — Sorcery
//! Oracle: Eternal Flame deals X damage to target opponent or planeswalker and half X damage, rounded up, to you, where X is the number of Mountains you control.
//! Set: DRK #61 — The Dark | Scryfall ID: d646feea-3c20-4737-8d20-ffad42258ced | Oracle ID: b3119f27-45fd-4411-8a09-5f8d3cd8d927

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_MOUNTAIN: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
]);

const MOUNTAINS_COUNT: Amount = Amount::CountOf {
    filter: &YOUR_MOUNTAIN,
    zone: ZoneSel::Battlefield,
};

card!(
    index = index::ETERNAL_FLAME,
    oracle_id = "b3119f27-45fd-4411-8a09-5f8d3cd8d927",
    scryfall_id = "d646feea-3c20-4737-8d20-ffad42258ced",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Eternal Flame",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: MOUNTAINS_COUNT,
                target: TargetSpec::OpponentOrObject(&Filter::PLANESWALKER),
            },
            Effect::DealDamage {
                amount: Amount::Half {
                    base: &MOUNTAINS_COUNT,
                    round_up: true,
                },
                target: TargetSpec::Player(PlayerRel::You),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::OpponentOrObject(
            &Filter::PLANESWALKER,
        ))),
    )],
);
