//! Banshee — {2}{B}{B} — Creature — Spirit
//! Oracle: {X}, {T}: This creature deals half X damage, rounded down, to any target, and half X damage, rounded up, to you.
//! Set: ME3 #59 — Masters Edition III | Scryfall ID: 12130a32-8c27-4e48-b1ae-b7991f751e7e | Oracle ID: 438546b8-0b67-4bec-a43b-f2fc7449c90e

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BANSHEE,
    oracle_id = "438546b8-0b67-4bec-a43b-f2fc7449c90e",
    scryfall_id = "12130a32-8c27-4e48-b1ae-b7991f751e7e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Banshee",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[
            Effect::DealDamage {
                amount: Amount::Half {
                    base: &Amount::X,
                    round_up: false,
                },
                target: TargetSpec::AnyTarget,
            },
            Effect::DealDamage {
                amount: Amount::Half {
                    base: &Amount::X,
                    round_up: true,
                },
                target: TargetSpec::Player(PlayerRel::You),
            },
        ],
        target = Some(TargetSpec::AnyTarget),
    )],
);
