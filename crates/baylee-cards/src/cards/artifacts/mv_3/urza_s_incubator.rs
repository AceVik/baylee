//! Urza's Incubator — {3} — Artifact
//! Oracle: As this artifact enters, choose a creature type.
//! Oracle: Creature spells of the chosen type cost {2} less to cast.
//! Set: MH3 #297 — Modern Horizons 3 | Scryfall ID: c4589226-6072-409b-b598-ebd87f675a6a | Oracle ID: b380c04b-0bce-4328-8f0a-1425e48bee72
// IMPLEMENTED — chooses a creature type as it enters (`EnterModifier::ChooseSubtype`)
// and reduces the cost of creature spells of the chosen type by {2} for all
// players (`Filter::MatchesChosenTypeOfSource` + `Modifier::SpellsCostLess(2)`).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_INCUBATOR,
    oracle_id = "b380c04b-0bce-4328-8f0a-1425e48bee72",
    scryfall_id = "c4589226-6072-409b-b598-ebd87f675a6a",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urza's Incubator",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]),
        Modifier::SpellsCostLess(2)
    )],
);
