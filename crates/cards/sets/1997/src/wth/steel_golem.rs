//! Steel Golem — {3} — Artifact Creature — Golem
//! Oracle: You can't cast creature spells.
//! Set: 10E #344 — Tenth Edition | Scryfall ID: 88d9b1ae-755d-49e4-b012-db95019491c6 | Oracle ID: b5cd9c55-34ad-4e1f-92b2-527c9babc4d4
// IMPLEMENTED — Prevents its controller from casting creature spells.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_CREATURE_SPELLS: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

card!(
    index = index::STEEL_GOLEM,
    oracle_id = "b5cd9c55-34ad-4e1f-92b2-527c9babc4d4",
    scryfall_id = "88d9b1ae-755d-49e4-b012-db95019491c6",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Steel Golem",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::PlayersCantPlay(&YOUR_CREATURE_SPELLS),
    ),],
);
