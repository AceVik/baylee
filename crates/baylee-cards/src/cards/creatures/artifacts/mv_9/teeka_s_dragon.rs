//! Teeka's Dragon — {9} — Artifact Creature — Dragon
//! Oracle: Flying; trample; rampage 4 (Whenever this creature becomes blocked, it gets +4/+4 until end of turn for each creature blocking it beyond the first.)
//! Set: MIR #320 — Mirage | Scryfall ID: 57e8971d-baeb-4e4f-8c4d-0e8109e4505e | Oracle ID: 6fb6e37e-d774-4a50-8ea3-b955535c2065
// IMPLEMENTED — flying, trample, and rampage 4.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEEKA_S_DRAGON,
    oracle_id = "6fb6e37e-d774-4a50-8ea3-b955535c2065",
    scryfall_id = "57e8971d-baeb-4e4f-8c4d-0e8109e4505e",
    keywords = KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Teeka's Dragon",
        mana_cost = mana!("{9}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(5),
        toughness = Some(5),
        keywords = KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
    ),],
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 4,
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 4,
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
