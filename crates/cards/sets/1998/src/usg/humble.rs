//! Humble — {1}{W} — Instant
//! Oracle: Until end of turn, target creature loses all abilities and has base power and toughness 0/1.
//! Set: USG #18 — Urza's Saga | Scryfall ID: 01383c7f-685f-4e77-a143-8418fe1fe436 | Oracle ID: d2d41f12-18d8-473b-9308-3f02d7d791f0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HUMBLE,
    oracle_id = "d2d41f12-18d8-473b-9308-3f02d7d791f0",
    scryfall_id = "01383c7f-685f-4e77-a143-8418fe1fe436",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Humble",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ModifyTargets {
                modifier: Modifier::LoseAllAbilities,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::SetPT(0, 1),
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
