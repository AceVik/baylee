//! Folk of An-Havva — {G} — Creature — Human
//! Oracle: Whenever this creature blocks, it gets +2/+0 until end of turn.
//! Set: HML #87a — Homelands | Scryfall ID: 9f4a595e-51f6-4e84-aa3a-5d9c18dc8b3f | Oracle ID: 86002526-ed3d-47e5-9f2a-55c4fa220ce8
// IMPLEMENTED — whenever this creature blocks, it gets +2/+0 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOLK_OF_AN_HAVVA,
    oracle_id = "86002526-ed3d-47e5-9f2a-55c4fa220ce8",
    scryfall_id = "9f4a595e-51f6-4e84-aa3a-5d9c18dc8b3f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Folk of An-Havva",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
);
