//! Copper Gnomes — {2} — Artifact Creature — Gnome
//! Oracle: {4}, Sacrifice this creature: You may put an artifact card from your hand onto the battlefield.
//! Set: USG #291 — Urza's Saga | Scryfall ID: d5e326b7-6f6a-4249-a315-c5f017931c73 | Oracle ID: 4fd6d9af-4d53-4e10-bc23-672454803099
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COPPER_GNOMES,
    oracle_id = "4fd6d9af-4d53-4e10-bc23-672454803099",
    scryfall_id = "d5e326b7-6f6a-4249-a315-c5f017931c73",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Copper Gnomes",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GNOME],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{4}", SacrificeSelf),
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::ARTIFACT,
            mana_value: None,
            optional: true,
        }],
    )],
);
