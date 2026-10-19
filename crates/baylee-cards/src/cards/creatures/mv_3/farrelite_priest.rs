//! Farrelite Priest — {1}{W}{W} — Creature — Human Cleric
//! Oracle: {1}: Add {W}. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.
//! Set: FEM #4 — Fallen Empires | Scryfall ID: e11bf79b-a951-4d0c-acdf-d8ba5290a648 | Oracle ID: 3ee18e5f-3d12-489e-b337-29208caaed7a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FARRELITE_PRIEST,
    oracle_id = "3ee18e5f-3d12-489e-b337-29208caaed7a",
    scryfall_id = "e11bf79b-a951-4d0c-acdf-d8ba5290a648",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Farrelite Priest",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[mana_ability!(
        cost!("{1}"),
        &[
            Effect::mana(ManaColor::White, 1),
            Effect::IfActivatedThisTurnAtLeast {
                n: 4,
                then: &[Effect::AtNextEndStep {
                    effects: &[Effect::SacrificeObject {
                        target: TargetSpec::EventObject,
                    }],
                }],
            },
        ],
    ),],
);
