//! Chronicle of Victory — {6} — Legendary Artifact
//! Oracle: As Chronicle of Victory enters, choose a creature type.
//! Oracle: Creatures you control of the chosen type get +2/+2 and have first strike and trample.
//! Oracle: Whenever you cast a spell of the chosen type, draw a card.
//! Set: ECL #253 — Lorwyn Eclipsed | Scryfall ID: b3c2d68d-690b-41e7-99ed-2d20c7e0a9b4 | Oracle ID: e949b362-4988-4482-ac51-c1049af9040c
// IMPLEMENTED — chosen-type static bonuses and cast triggers, including Kindred spells.

use crate::filters::{YOUR_CREATURE_OF_CHOSEN_TYPE, YOUR_SPELL_OF_CHOSEN_TYPE};
use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHRONICLE_OF_VICTORY,
    oracle_id = "e949b362-4988-4482-ac51-c1049af9040c",
    scryfall_id = "b3c2d68d-690b-41e7-99ed-2d20c7e0a9b4",
    faces = &[face!(
        name = "Chronicle of Victory",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(YOUR_CREATURE_OF_CHOSEN_TYPE, Modifier::ModifyPT(2, 2)),
        static_ability!(
            YOUR_CREATURE_OF_CHOSEN_TYPE,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE.union(KeywordSet::TRAMPLE))
        ),
        triggered!(
            Trigger::SpellCast(&YOUR_SPELL_OF_CHOSEN_TYPE),
            &[Effect::DrawCards {
                amount: Amount::Fixed(1)
            }]
        ),
    ],
);
