//! Arc Lightning — {2}{R} — Sorcery
//! Oracle: Arc Lightning deals 3 damage divided as you choose among one, two, or three targets.
//! Set: KTK #97 — Khans of Tarkir | Scryfall ID: 35c7c392-6782-40c8-bb24-6aad24f14660 | Oracle ID: 5acc8b39-3c3e-4012-8cfd-ac3c2c4ca982
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARC_LIGHTNING,
    oracle_id = "5acc8b39-3c3e-4012-8cfd-ac3c2c4ca982",
    scryfall_id = "35c7c392-6782-40c8-bb24-6aad24f14660",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arc Lightning",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::Fixed(3),
        }],
        targets = Some(TargetReq {
            spec: TargetSpec::AnyTarget,
            min: 1,
            max: 3,
            count_is_x: false,
            distinct_mana_values: false,
            chooser: TargetChooser::Controller,
        }),
    )],
);
