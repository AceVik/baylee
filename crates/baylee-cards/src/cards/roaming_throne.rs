//! Roaming Throne — {4} — Artifact Creature — Golem
//! Oracle: Ward {2}
//! Oracle: As this creature enters, choose a creature type.
//! Oracle: This creature is the chosen type in addition to its other types.
//! Oracle: If a triggered ability of another creature you control of the chosen type triggers, it triggers an additional time.
//! Set: LCI #258 — The Lost Caverns of Ixalan | Scryfall ID: 32fd8b7c-baf3-4d3d-be6f-044a917b11a0 | Oracle ID: 3640c29b-1534-4952-b297-619ade948431
// IMPLEMENTED — ward {2} (synthetic trigger), choose-a-type on entry
// (gains the subtype), and the chosen-type trigger multiplier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static OTHER_CHOSEN_TYPE_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Another,
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::MatchesChosenTypeOfSource,
]);

card! {
    index: 136,
    oracle_id: "3640c29b-1534-4952-b297-619ade948431",
    scryfall_id: "32fd8b7c-baf3-4d3d-be6f-044a917b11a0",
    faces: &[face! {
        name: "Roaming Throne",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::GOLEM],
        power: Some(4),
        toughness: Some(4),
        enter_modifiers: &[EnterModifier::ChooseSubtype],
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Ward { mana: 2 },
        AbilityDef::Replacement(ReplacementRule::TriggerMultiplier {
            source_filter: &OTHER_CHOSEN_TYPE_CREATURE_YOU_CONTROL,
            event: TriggerEventKind::Any,
        }),
    ],
}
