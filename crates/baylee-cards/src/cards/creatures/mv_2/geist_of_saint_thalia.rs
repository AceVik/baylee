//! Geist of Saint Thalia — {1}{U} — Legendary Creature — Spirit Cleric
//! Oracle: Flying
//! Oracle: Noncreature spells you cast cost {1} less to cast.
//! Set: FRA #214 — Reality Fracture | Scryfall ID: 9c334530-0880-46b5-a358-9603eee3cecf | Oracle ID: ef32a4a9-14e2-4738-b4c2-53ce5e1d2a53
// PARTIAL — flying only; there is no cost-reducer vocabulary in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEIST_OF_SAINT_THALIA,
    oracle_id = "ef32a4a9-14e2-4738-b4c2-53ce5e1d2a53",
    scryfall_id = "9c334530-0880-46b5-a358-9603eee3cecf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Geist of Saint Thalia",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SPIRIT, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Noncreature spells you cast cost {1} less to cast: no Modifier or Effect \
         variant reduces a spell's cost, and CostReduction carries only \
         NotStartingPlayer"
    ),
);

// NOT SUPPORTED: Noncreature spells you cast cost {1} less to cast.
