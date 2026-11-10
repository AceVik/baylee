//! Planar Collapse — {1}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, if there are four or more creatures on the battlefield, sacrifice this enchantment and destroy all creatures. They can't be regenerated.
//! Set: ULG #18 — Urza's Legacy | Scryfall ID: ee22cf3c-51b0-4790-ab13-985cbe900c3b | Oracle ID: 82bccc9f-f700-4f1f-b89e-bea63d973161
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_COLLAPSE,
    oracle_id = "82bccc9f-f700-4f1f-b89e-bea63d973161",
    scryfall_id = "ee22cf3c-51b0-4790-ab13-985cbe900c3b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Planar Collapse",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::SacrificeSelf,
            Effect::destroy_all_no_regen(&Filter::CREATURE),
        ],
        condition = Some(Condition::BattlefieldCount(&Filter::CREATURE, 4)),
    )],
);
