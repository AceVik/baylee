//! Deflection — {3}{U} — Instant
//! Oracle: Change the target of target spell with a single target.
//! Set: 8ED #74 — Eighth Edition | Scryfall ID: 217c899e-f624-4791-af64-b1b1e2ac53cd | Oracle ID: ec7ae9ed-dc5b-47ed-a4ad-086f3c7c377c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFLECTION,
    oracle_id = "ec7ae9ed-dc5b-47ed-a4ad-086f3c7c377c",
    scryfall_id = "217c899e-f624-4791-af64-b1b1e2ac53cd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deflection",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ChangeTarget { to: &Filter::Any }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::WithSingleTarget))),
    )],
);
