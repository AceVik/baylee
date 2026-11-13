//! Viashino Heretic — {2}{R} — Creature — Lizard
//! Oracle: {1}{R}, {T}: Destroy target artifact. This creature deals damage to that artifact's controller equal to the artifact's mana value.
//! Set: ULG #95 — Urza's Legacy | Scryfall ID: 143e435e-e3a1-45b0-81c3-bd47916df8ac | Oracle ID: 36896550-151d-472c-bfc1-1d9bbf7cc04d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_HERETIC,
    oracle_id = "36896550-151d-472c-bfc1-1d9bbf7cc04d",
    scryfall_id = "143e435e-e3a1-45b0-81c3-bd47916df8ac",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viashino Heretic",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}", TapSelf),
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT)),
            Effect::DealDamageToTargetController {
                amount: Amount::TargetCmc,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    )],
);
