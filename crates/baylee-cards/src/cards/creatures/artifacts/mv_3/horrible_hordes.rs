//! Horrible Hordes — {3} — Artifact Creature — Spirit
//! Oracle: Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: MIR #304 — Mirage | Scryfall ID: cf04f2cd-d9f8-4e0c-adaf-6d38bc14dd7a | Oracle ID: 79bfef02-3586-424a-aff9-a6ec0d4f1135
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HORRIBLE_HORDES,
    oracle_id = "79bfef02-3586-424a-aff9-a6ec0d4f1135",
    scryfall_id = "cf04f2cd-d9f8-4e0c-adaf-6d38bc14dd7a",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Horrible Hordes",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::BlockersBeyondFirst,
            toughness: Amount::BlockersBeyondFirst,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
