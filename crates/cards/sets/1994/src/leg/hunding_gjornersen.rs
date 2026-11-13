//! Hunding Gjornersen — {3}{W}{U}{U} — Legendary Creature — Human Warrior
//! Oracle: Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #152 — Masters Edition III | Scryfall ID: 4049abac-cb54-4af2-b2df-ffb8fdc22e84 | Oracle ID: dd4f3a78-3167-42d0-8334-5ac36750a253

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNDING_GJORNERSEN,
    oracle_id = "dd4f3a78-3167-42d0-8334-5ac36750a253",
    scryfall_id = "4049abac-cb54-4af2-b2df-ffb8fdc22e84",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hunding Gjornersen",
        mana_cost = mana!("{3}{W}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 1
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 1
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    ),],
);
