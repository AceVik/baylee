//! Dwarven Song — {R} — Instant
//! Oracle: One or more target creatures become red until end of turn.
//! Set: LEG #143 — Legends | Scryfall ID: 29a50f72-9524-4440-9380-9d3e0b693351 | Oracle ID: 29b7a220-cfcf-4b44-a5fb-737fd47d46cb

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWARVEN_SONG,
    oracle_id = "29b7a220-cfcf-4b44-a5fb-737fd47d46cb",
    scryfall_id = "29a50f72-9524-4440-9380-9d3e0b693351",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::SetColor(ColorSet::from_slice(&[Color::Red])),
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
        name = "Dwarven Song",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);
