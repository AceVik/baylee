//! Glyph of Life — {W} — Instant
//! Oracle: Choose target Wall creature. Whenever that creature is dealt damage by an attacking creature this turn, you gain that much life.
//! Set: LEG #15 — Legends | Scryfall ID: ba1384e5-d140-4074-9548-250af09cb413 | Oracle ID: ec450179-35e0-4d72-b42f-b507cbce03ad
// IMPLEMENTED — a delayed trigger for the rest of the turn: each time the
// target Wall is dealt damage by an attacking creature, the caster gains that
// much life (the Wall still takes the damage).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Target Wall creature."
static WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

card!(
    index = index::GLYPH_OF_LIFE,
    oracle_id = "ec450179-35e0-4d72-b42f-b507cbce03ad",
    scryfall_id = "ba1384e5-d140-4074-9548-250af09cb413",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Glyph of Life",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::WheneverTargetDealtDamageThisTurn {
            by: &Filter::ATTACKING_CREATURE,
            effects: &[Effect::GainLife {
                amount: Amount::EventAmount
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&WALL)))
    )],
);
