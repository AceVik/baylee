//! Second Chance — {2}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, if you have 5 or less life, sacrifice this enchantment and take an extra turn after this one.
//! Set: ULG #41 — Urza's Legacy | Scryfall ID: 62d1a0da-40b9-4e79-bace-b93f98ae4695 | Oracle ID: aa19bbf0-7de3-4ec3-a98b-2ac7533c6b0b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SECOND_CHANCE,
    oracle_id = "aa19bbf0-7de3-4ec3-a98b-2ac7533c6b0b",
    scryfall_id = "62d1a0da-40b9-4e79-bace-b93f98ae4695",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Second Chance",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::SacrificeSelf, Effect::TakeExtraTurn],
        condition = Some(Condition::LifeAtMost(5)),
    )],
);
