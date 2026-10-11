//! Indestructible Aura — {W} — Instant
//! Oracle: Prevent all damage that would be dealt to target creature this turn.
//! Set: CHR #7 — Chronicles | Scryfall ID: 0397a4f3-6d7c-43d1-9fc2-c0eaf780ecb0 | Oracle ID: e10e8d56-bba6-412d-970e-c24969f32b5b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INDESTRUCTIBLE_AURA,
    oracle_id = "e10e8d56-bba6-412d-970e-c24969f32b5b",
    scryfall_id = "0397a4f3-6d7c-43d1-9fc2-c0eaf780ecb0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Indestructible Aura",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::PreventDamageFrom(&Filter::Any),
            duration: Duration::UntilEndOfTurn
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
