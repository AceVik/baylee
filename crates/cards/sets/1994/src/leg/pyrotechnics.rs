//! Pyrotechnics — {4}{R} — Sorcery
//! Oracle: Pyrotechnics deals 4 damage divided as you choose among any number of targets.
//! Set: FRF #111 — Fate Reforged | Scryfall ID: 51893dd5-e70f-44bb-85d4-e31480ba84d6 | Oracle ID: 794b9ddf-1660-441f-9cc2-a0f5d4d0cf22

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PYROTECHNICS,
    oracle_id = "794b9ddf-1660-441f-9cc2-a0f5d4d0cf22",
    scryfall_id = "51893dd5-e70f-44bb-85d4-e31480ba84d6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pyrotechnics",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::Fixed(4)
        }],
        targets = Some(TargetReq::up_to(TargetSpec::AnyTarget, 4)),
    )],
);
