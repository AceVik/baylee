//! Wolverine Pack — {2}{G}{G} — Creature — Wolverine
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: 5ED #344 — Fifth Edition | Scryfall ID: fab6d1e8-0985-4560-aea2-7ad1925a2f5a | Oracle ID: d67d9d82-2786-48fc-8107-11d12c5813d0

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WOLVERINE_PACK,
    oracle_id = "d67d9d82-2786-48fc-8107-11d12c5813d0",
    scryfall_id = "fab6d1e8-0985-4560-aea2-7ad1925a2f5a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 2
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 2
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
    faces = &[face!(
        name = "Wolverine Pack",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLVERINE],
        power = Some(2),
        toughness = Some(4),
    ),],
);
