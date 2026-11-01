//! Raging Spirit — {3}{R} — Creature — Spirit
//! Oracle: {2}: This creature becomes colorless until end of turn.
//! Set: MIR #188 — Mirage | Scryfall ID: a196c21b-e9f5-4ae8-8a1e-668685ef4cf0 | Oracle ID: 137e0ad2-e10a-4327-86a3-5ac7bc7fc469
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAGING_SPIRIT,
    oracle_id = "137e0ad2-e10a-4327-86a3-5ac7bc7fc469",
    scryfall_id = "a196c21b-e9f5-4ae8-8a1e-668685ef4cf0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Raging Spirit",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[Effect::ModifySource {
            modifier: Modifier::SetColor(ColorSet::EMPTY),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
