//! Johtull Wurm — {5}{G} — Creature — Wurm
//! Oracle: Whenever this creature becomes blocked, it gets -2/-1 until end of turn for each creature blocking it beyond the first.
//! Set: ME2 #168 — Masters Edition II | Scryfall ID: ff4f2361-2f99-41f5-b295-ca26c9eee27c | Oracle ID: ab5a8242-4e42-43e6-a6ee-eeb0a8db3d11

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JOHTULL_WURM,
    oracle_id = "ab5a8242-4e42-43e6-a6ee-eeb0a8db3d11",
    scryfall_id = "ff4f2361-2f99-41f5-b295-ca26c9eee27c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Johtull Wurm",
        mana_cost = mana!("{5}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Negated(&Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 2,
            }),
            toughness: Amount::Negated(&Amount::BlockersBeyondFirst),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
