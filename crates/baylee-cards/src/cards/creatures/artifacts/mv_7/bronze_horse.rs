//! Bronze Horse — {7} — Artifact Creature — Horse
//! Oracle: Trample
//! Oracle: As long as you control another creature, prevent all damage that would be dealt to this creature by spells that target it.
//! Set: ME4 #186 — Masters Edition IV | Scryfall ID: 0003b07e-0d6e-4844-93c7-3f1f6a7d8c4d | Oracle ID: a7887b24-977d-4a40-bb30-4bf467e5dba6

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRONZE_HORSE,
    oracle_id = "a7887b24-977d-4a40-bb30-4bf467e5dba6",
    scryfall_id = "0003b07e-0d6e-4844-93c7-3f1f6a7d8c4d",
    faces = &[face!(
        name = "Bronze Horse",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::HORSE],
        power = Some(4),
        toughness = Some(4),
    ),],
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PreventDamageFrom(&Filter::SpellTargetingAffected),
        condition = Some(Condition::Any(&[
            Condition::All(&[
                Condition::SourceMatches(&Filter::CREATURE),
                Condition::ControlCount(&Filter::CREATURE, 2)
            ]),
            Condition::All(&[
                Condition::Not(&Condition::SourceMatches(&Filter::CREATURE)),
                Condition::ControlCount(&Filter::CREATURE, 1)
            ]),
        ]))
    )],
);
