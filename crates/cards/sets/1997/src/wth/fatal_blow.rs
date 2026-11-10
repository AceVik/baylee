//! Fatal Blow — {B} — Instant
//! Oracle: Destroy target creature that was dealt damage this turn. It can't be regenerated.
//! Set: 6ED #128 — Classic Sixth Edition | Scryfall ID: 6890c3aa-9321-4c41-9b16-cff4e6364350 | Oracle ID: 16774611-c004-4383-b4e1-4a43a9b1f4aa
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static DAMAGED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::DealtDamageThisTurn]);

card!(
    index = index::FATAL_BLOW,
    oracle_id = "16774611-c004-4383-b4e1-4a43a9b1f4aa",
    scryfall_id = "6890c3aa-9321-4c41-9b16-cff4e6364350",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fatal Blow",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy_no_regen(TargetSpec::Object(
            &DAMAGED_CREATURE
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(&DAMAGED_CREATURE))),
    )],
);
