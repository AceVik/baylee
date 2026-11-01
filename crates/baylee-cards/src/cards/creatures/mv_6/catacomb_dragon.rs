//! Catacomb Dragon — {4}{B}{B} — Creature — Dragon
//! Oracle: Flying
//! Oracle: Whenever this creature becomes blocked by a nonartifact, non-Dragon creature, that creature gets -X/-0 until end of turn, where X is half the creature's power, rounded down.
//! Set: MIR #112 — Mirage | Scryfall ID: 46daebe4-199e-4580-8a52-1aebc8492d8c | Oracle ID: a5e9f72b-7628-4265-8b43-592a0bf678b3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NONARTIFACT_NONDRAGON_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::LacksType(TypeSet::ARTIFACT),
    Filter::Not(&Filter::HasSubtype(subtypes::creature::DRAGON)),
]);

card!(
    index = index::CATACOMB_DRAGON,
    oracle_id = "a5e9f72b-7628-4265-8b43-592a0bf678b3",
    scryfall_id = "46daebe4-199e-4580-8a52-1aebc8492d8c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Catacomb Dragon",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&NONARTIFACT_NONDRAGON_CREATURE),
        &[Effect::PumpFilter {
            filter: &Filter::Reference,
            controlled_by: None,
            power: Amount::Negated(&Amount::Half {
                base: &Amount::EventPower,
                round_up: false,
            }),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
