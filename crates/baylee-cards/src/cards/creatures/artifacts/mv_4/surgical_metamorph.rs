//! Surgical Metamorph — {3}{U} — Artifact Creature — Phyrexian Shapeshifter
//! Oracle: This spell costs {1} less to cast if you weren't the starting player.
//! Oracle: You may have Surgical Metamorph enter as a copy of any permanent on the battlefield, except it's an artifact in addition to its other types.
//! Set: YONE #6 — Alchemy: Phyrexia | Scryfall ID: 1e7aa3a6-4219-4c54-97bd-571680af9e99 | Oracle ID: 4f328996-f9dd-4c7a-9548-bc4b9d0d943f
// IMPLEMENTED — clone-of-any-permanent + the not-starting-player cost
// reduction (FaceDef::cost_reduction).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 161,
    oracle_id: "4f328996-f9dd-4c7a-9548-bc4b9d0d943f",
    scryfall_id: "1e7aa3a6-4219-4c54-97bd-571680af9e99",
    faces: &[face! {
        name: "Surgical Metamorph",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::PHYREXIAN, creature::SHAPESHIFTER],
        power: Some(0),
        toughness: Some(0),
        cost_reduction: Some(CostReduction::NotStartingPlayer(1)),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::Any),
        mods: &[CopyMod::AddType(TypeSet::ARTIFACT)],
    }],
}
