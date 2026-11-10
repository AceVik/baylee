//! Colossus of Sardia — {9} — Artifact Creature — Golem
//! Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: {9}: Untap this creature. Activate only during your upkeep.
//! Set: ME4 #193 — Masters Edition IV | Scryfall ID: d0926087-73cf-4aa4-80f9-6f2c54e230ec | Oracle ID: 9be9625e-b98b-416b-aac4-9f7b2dfbd39d
// IMPLEMENTED — trample, a layer-6 untap suppression, and the {9} untap activatable only during your upkeep.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COLOSSUS_OF_SARDIA,
    oracle_id = "9be9625e-b98b-416b-aac4-9f7b2dfbd39d",
    scryfall_id = "d0926087-73cf-4aa4-80f9-6f2c54e230ec",
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Colossus of Sardia",
        mana_cost = mana!("{9}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(9),
        toughness = Some(9),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        activated!(
            cost!("{9}"),
            &[Effect::UntapSelf],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
