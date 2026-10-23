//! Gang of Elk — {5}{G} — Creature — Elk Beast
//! Oracle: Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it.
//! Set: 7ED #247 — Seventh Edition | Scryfall ID: cd0a61c9-8b14-4255-8453-4b74d90fe0a3 | Oracle ID: b99d6d51-3ead-4b81-8e0c-f458619ebb24
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKING_CREATURES: Amount = Amount::CountOf {
    filter: &Filter::And(&[Filter::CREATURE, Filter::BlockingSource]),
    zone: ZoneSel::Battlefield,
};

static TWICE_BLOCKERS: Amount = Amount::Times {
    base: &BLOCKING_CREATURES,
    factor: 2,
};

card!(
    index = index::GANG_OF_ELK,
    oracle_id = "b99d6d51-3ead-4b81-8e0c-f458619ebb24",
    scryfall_id = "cd0a61c9-8b14-4255-8453-4b74d90fe0a3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gang of Elk",
        mana_cost = mana!("{5}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELK, subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: TWICE_BLOCKERS,
            toughness: TWICE_BLOCKERS,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
