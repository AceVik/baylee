//! Eladamri's Vineyard — {G} — Enchantment
//! Oracle: At the beginning of each player's first main phase, that player adds {G}{G}.
//! Set: TMP #223 — Tempest | Scryfall ID: d8531643-5657-44b6-89d1-9cdf67ed09c4 | Oracle ID: faa085c3-705f-465d-9290-0e22276ad06d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELADAMRI_S_VINEYARD,
    oracle_id = "faa085c3-705f-465d-9290-0e22276ad06d",
    scryfall_id = "d8531643-5657-44b6-89d1-9cdf67ed09c4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Eladamri's Vineyard",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::FirstMain,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::AddManaFor {
            who: PlayerRel::ActivePlayer,
            color: ManaColor::Green,
            amount: 2,
        }],
    )],
);
