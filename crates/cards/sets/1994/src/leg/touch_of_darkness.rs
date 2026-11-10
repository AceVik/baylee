//! Touch of Darkness — {B} — Instant
//! Oracle: One or more target creatures become black until end of turn.
//! Set: LEG #122 — Legends | Scryfall ID: eda7177f-1354-4008-aaaa-2c8b823ed5e9 | Oracle ID: 73648693-c67f-454b-83e2-1d1787437a2a

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOUCH_OF_DARKNESS,
    oracle_id = "73648693-c67f-454b-83e2-1d1787437a2a",
    scryfall_id = "eda7177f-1354-4008-aaaa-2c8b823ed5e9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::SetColor(ColorSet::from_slice(&[Color::Black])),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq {
            spec: TargetSpec::Object(&Filter::CREATURE),
            min: 1,
            max: u8::MAX,
            count_is_x: false,
            distinct_mana_values: false,
            chooser: baylee_cards_dsl::TargetChooser::Controller,
        })
    )],
    faces = &[face!(
        name = "Touch of Darkness",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
);
