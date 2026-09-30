//! Berserk — {G} — Instant
//! Oracle: Cast this spell only before the combat damage step.
//! Oracle: Target creature gains trample and gets +X/+0 until end of turn, where X is its power. At the beginning of the next end step, destroy that creature if it attacked this turn.
//! Set: CN2 #175 — Conspiracy: Take the Crown | Scryfall ID: 62bc9bff-89bd-4454-a876-53822cf48546 | Oracle ID: 8b67d192-9a05-4a47-82ae-5fc4b7834d88
// PARTIAL — the timing restriction and the end-step destruction are not in the
// engine; it grants trample and +X/+0.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BERSERK,
    oracle_id = "8b67d192-9a05-4a47-82ae-5fc4b7834d88",
    scryfall_id = "62bc9bff-89bd-4454-a876-53822cf48546",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the timing restriction and the end-step destruction are not in the engine; it grants trample and +X/+0"
    ),
    faces = &[face!(
        name = "Berserk",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::PumpTarget {
                power: Amount::TargetPower,
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::TRAMPLE,
                duration: Duration::UntilEndOfTurn
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        // NOT SUPPORTED: Cast this spell only before the combat damage step.
        // NOT SUPPORTED: At the beginning of the next end step, destroy that creature if
        // it attacked this turn.
    ],
);
