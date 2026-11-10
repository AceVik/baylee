//! Svyelunite Priest — {1}{U} — Creature — Merfolk Cleric
//! Oracle: {U}{U}, {T}: Target creature gains shroud until end of turn. Activate only during your upkeep. (It can't be the target of spells or abilities.)
//! Set: FEM #26 — Fallen Empires | Scryfall ID: 316d25ae-7ac6-4f5b-93ab-0e0e28ec104b | Oracle ID: fb7adbee-a3b0-42bf-988f-9d4faf6bd006
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SVYELUNITE_PRIEST,
    oracle_id = "fb7adbee-a3b0-42bf-988f-9d4faf6bd006",
    scryfall_id = "316d25ae-7ac6-4f5b-93ab-0e0e28ec104b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Svyelunite Priest",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{U}{U}", TapSelf),
        &[Effect::ModifyTargets {
            modifier: Modifier::AddKeyword(KeywordSet::SHROUD),
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
        ])),
    ),],
);
