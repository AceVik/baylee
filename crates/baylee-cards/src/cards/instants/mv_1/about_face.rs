//! About Face — {R} — Instant
//! Oracle: Switch target creature's power and toughness until end of turn.
//! Set: ULG #73 — Urza's Legacy | Scryfall ID: 85e71828-095b-4729-ab11-c6c39ba29aab | Oracle ID: 8a810017-06ac-4824-bc2f-a7d303d58ad7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABOUT_FACE,
    oracle_id = "8a810017-06ac-4824-bc2f-a7d303d58ad7",
    scryfall_id = "85e71828-095b-4729-ab11-c6c39ba29aab",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "About Face",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::continuous(
            &Filter::This,
            Modifier::SwitchPT,
            Duration::UntilEndOfTurn,
        )],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
