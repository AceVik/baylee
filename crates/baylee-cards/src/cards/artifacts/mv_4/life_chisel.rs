//! Life Chisel — {4} — Artifact
//! Oracle: Sacrifice a creature: You gain life equal to the sacrificed creature's toughness. Activate only during your upkeep.
//! Set: ME3 #199 — Masters Edition III | Scryfall ID: 6d06edbe-2278-4818-b476-75bc02958418 | Oracle ID: 2493fe65-2ecb-418f-8e4e-797f83475c73
// IMPLEMENTED — gains life equal to the sacrificed creature's toughness,
// only during your upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE_CHISEL,
    oracle_id = "2493fe65-2ecb-418f-8e4e-797f83475c73",
    scryfall_id = "6d06edbe-2278-4818-b476-75bc02958418",
    faces = &[face!(
        name = "Life Chisel",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::CREATURE)),
        &[Effect::GainLife {
            amount: Amount::SacrificedToughness
        }],
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
        ])),
    )],
);
