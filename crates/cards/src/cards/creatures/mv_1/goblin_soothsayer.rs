//! Goblin Soothsayer — {R} — Creature — Goblin Shaman
//! Oracle: {R}, {T}, Sacrifice a Goblin: Red creatures get +1/+1 until end of turn.
//! Set: MIR #179 — Mirage | Scryfall ID: 686b847c-242c-4b91-9fa9-69c9c9f187a7 | Oracle ID: a9277df9-1417-45bc-bebf-d9a7595f54c9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PUMP_ALL1: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

static COST2: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::ControlledByYou,
]);

card!(
    index = index::GOBLIN_SOOTHSAYER,
    oracle_id = "a9277df9-1417-45bc-bebf-d9a7595f54c9",
    scryfall_id = "686b847c-242c-4b91-9fa9-69c9c9f187a7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Soothsayer",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{R}", TapSelf, Sacrifice(&COST2)),
        &[Effect::PumpFilter {
            filter: &PUMP_ALL1,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
