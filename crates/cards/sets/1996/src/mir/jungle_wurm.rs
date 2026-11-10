//! Jungle Wurm — {3}{G}{G} — Creature — Wurm
//! Oracle: Whenever this creature becomes blocked, it gets -1/-1 until end of turn for each creature blocking it beyond the first.
//! Set: VMA #217 — Vintage Masters | Scryfall ID: f646c83f-e804-4878-9160-8f2059dba496 | Oracle ID: 457705c9-5b0c-4c3a-89dd-80286fac6a6c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JUNGLE_WURM,
    oracle_id = "457705c9-5b0c-4c3a-89dd-80286fac6a6c",
    scryfall_id = "f646c83f-e804-4878-9160-8f2059dba496",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jungle Wurm",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Negated(&Amount::BlockersBeyondFirst),
            toughness: Amount::Negated(&Amount::BlockersBeyondFirst),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
