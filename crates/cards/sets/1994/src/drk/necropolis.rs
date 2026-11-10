//! Necropolis — {5} — Artifact Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Exile a creature card from your graveyard: Put X +0/+1 counters on this creature, where X is the exiled card's mana value.
//! Set: DRK #105 — The Dark | Scryfall ID: 893e8e9c-983e-4db1-8d93-10637025a559 | Oracle ID: c84adca4-19dd-432d-a363-e98d3b699af8

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NECROPOLIS,
    oracle_id = "c84adca4-19dd-432d-a363-e98d3b699af8",
    scryfall_id = "893e8e9c-983e-4db1-8d93-10637025a559",
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Necropolis",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(ExileFromGraveyard(&Filter::CREATURE)),
        &[Effect::AddCounter {
            kind: CounterKind::Plus {
                power: 0,
                toughness: 1,
            },
            amount: Amount::SacrificedManaValue,
        }]
    )],
);
