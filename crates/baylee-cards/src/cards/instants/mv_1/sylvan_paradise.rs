//! Sylvan Paradise — {G} — Instant
//! Oracle: One or more target creatures become green until end of turn.
//! Set: LEG #208 — Legends | Scryfall ID: f323c3bb-cece-4035-b1a7-c4817cf7a08c | Oracle ID: 91693cad-3233-440a-b2f0-fe4d8b17ed42

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYLVAN_PARADISE,
    oracle_id = "91693cad-3233-440a-b2f0-fe4d8b17ed42",
    scryfall_id = "f323c3bb-cece-4035-b1a7-c4817cf7a08c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::SetColor(ColorSet::from_slice(&[Color::Green])),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq {
            spec: TargetSpec::Object(&Filter::CREATURE),
            min: 1,
            max: u8::MAX,
            count_is_x: false,
            distinct_mana_values: false,
        })
    )],
    faces = &[face!(
        name = "Sylvan Paradise",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);
