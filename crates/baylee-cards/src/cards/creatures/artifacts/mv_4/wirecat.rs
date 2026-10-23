//! Wirecat — {4} — Artifact Creature — Cat
//! Oracle: This creature can't attack or block if an enchantment is on the battlefield.
//! Set: USG #317 — Urza's Saga | Scryfall ID: 6333d686-58ec-4360-8929-9f7302f9a09c | Oracle ID: 57828973-df3d-4288-988d-147a3016120e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WIRECAT,
    oracle_id = "57828973-df3d-4288-988d-147a3016120e",
    scryfall_id = "6333d686-58ec-4360-8929-9f7302f9a09c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wirecat",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CAT],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)),
        condition = Some(Condition::BattlefieldCount(&Filter::ENCHANTMENT, 1)),
    )],
);
