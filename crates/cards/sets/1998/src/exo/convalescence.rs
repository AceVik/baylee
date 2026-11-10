//! Convalescence — {1}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, if you have 10 or less life, you gain 1 life.
//! Set: EXO #5 — Exodus | Scryfall ID: 0fd49a61-42ba-400a-8ca9-9f6058bf85ca | Oracle ID: 9ad86d0e-3363-4d98-b0bb-dc491be1ddd5
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONVALESCENCE,
    oracle_id = "9ad86d0e-3363-4d98-b0bb-dc491be1ddd5",
    scryfall_id = "0fd49a61-42ba-400a-8ca9-9f6058bf85ca",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Convalescence",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::gain_life(1)],
        condition = Some(Condition::LifeAtMost(10)),
    )],
);
