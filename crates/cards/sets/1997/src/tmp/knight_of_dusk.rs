//! Knight of Dusk — {1}{B}{B} — Creature — Human Knight
//! Oracle: {B}{B}: Destroy target creature blocking this creature.
//! Set: 10E #152 — Tenth Edition | Scryfall ID: d665187f-a8d3-40b6-bfaf-cf25d84484b4 | Oracle ID: cd1ca3c8-1f03-4c03-a93a-0419ac205c18
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_THIS: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockingSource]);

card!(
    index = index::KNIGHT_OF_DUSK,
    oracle_id = "cd1ca3c8-1f03-4c03-a93a-0419ac205c18",
    scryfall_id = "d665187f-a8d3-40b6-bfaf-cf25d84484b4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Knight of Dusk",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{B}{B}"),
        &[Effect::destroy(TargetSpec::Object(&BLOCKING_THIS))],
        target = Some(TargetSpec::Object(&BLOCKING_THIS)),
    )],
);
