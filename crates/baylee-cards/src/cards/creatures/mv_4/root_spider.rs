//! Root Spider — {3}{G} — Creature — Spider
//! Oracle: Whenever this creature blocks, it gets +1/+0 and gains first strike until end of turn.
//! Set: HML #94 — Homelands | Scryfall ID: 407d67b0-d496-401b-8844-8e3ea2fd2046 | Oracle ID: d8fe0115-5d98-4251-a48c-5e849430a445
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOT_SPIDER,
    oracle_id = "d8fe0115-5d98-4251-a48c-5e849430a445",
    scryfall_id = "407d67b0-d496-401b-8844-8e3ea2fd2046",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Root Spider",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIDER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::FIRST_STRIKE,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
);
