//! Verdeloth the Ancient — {4}{G}{G} — Legendary Creature — Treefolk
//! Oracle: Kicker {X} (You may pay an additional {X} as you cast this spell.)
//! Oracle: Saproling creatures and other Treefolk creatures get +1/+1.
//! Oracle: When Verdeloth enters, if it was kicked, create X 1/1 green Saproling creature tokens.
//! Set: J25 #737 — Foundations Jumpstart | Scryfall ID: 4b3e632e-21dc-4cc3-9c80-4b3d8b8e5795 | Oracle ID: 8222321f-b74d-4f74-9ee7-79455433345a
// IMPLEMENTED — Saproling and other Treefolk lord with kicker {X} Saproling token ETB trigger.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SAPROLING_AND_OTHER_TREEFOLK: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::SAPROLING),
        Filter::And(&[
            Filter::HasSubtype(subtypes::creature::TREEFOLK),
            Filter::Another,
        ]),
    ]),
]);

card!(
    index = index::VERDELOTH_THE_ANCIENT,
    oracle_id = "8222321f-b74d-4f74-9ee7-79455433345a",
    scryfall_id = "4b3e632e-21dc-4cc3-9c80-4b3d8b8e5795",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Verdeloth the Ancient",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(4),
        toughness = Some(7),
        additional_costs = &[cost!("{X}")],
    ),],
    abilities = &[
        static_ability!(SAPROLING_AND_OTHER_TREEFOLK, Modifier::ModifyPT(1, 1)),
        triggered!(
            Trigger::ETB,
            &[Effect::CreateTokenN {
                token: &generated_tokens::SAPROLING_1_1_GREEN,
                amount: Amount::X,
            }],
            condition = Some(Condition::KickedWith(0))
        ),
    ],
);
