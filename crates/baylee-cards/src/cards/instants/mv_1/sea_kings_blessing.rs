//! Sea Kings' Blessing — {U} — Instant
//! Oracle: One or more target creatures become blue until end of turn.
//! Set: LEG #75 — Legends | Scryfall ID: 11d1f02d-533e-4b77-a72a-ff5f91ae0626 | Oracle ID: e6ea475e-61ba-487b-b7e9-683845a90b73

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEA_KINGS_BLESSING,
    oracle_id = "e6ea475e-61ba-487b-b7e9-683845a90b73",
    scryfall_id = "11d1f02d-533e-4b77-a72a-ff5f91ae0626",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::SetColor(ColorSet::from_slice(&[Color::Blue])),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq {
            spec: TargetSpec::Object(&Filter::CREATURE),
            min: 1,
            max: u8::MAX,
            count_is_x: false
        })
    )],
    faces = &[face!(
        name = "Sea Kings' Blessing",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);
