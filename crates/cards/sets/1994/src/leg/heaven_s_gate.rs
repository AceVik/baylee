//! Heaven's Gate — {W} — Instant
//! Oracle: One or more target creatures become white until end of turn.
//! Set: LEG #19 — Legends | Scryfall ID: 461d7c11-3a7d-42c2-bb6b-0a43779e6842 | Oracle ID: fafdba94-51cd-413c-82f3-eb294825c6ca

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEAVEN_S_GATE,
    oracle_id = "fafdba94-51cd-413c-82f3-eb294825c6ca",
    scryfall_id = "461d7c11-3a7d-42c2-bb6b-0a43779e6842",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::SetColor(ColorSet::from_slice(&[Color::White])),
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
        name = "Heaven's Gate",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);
