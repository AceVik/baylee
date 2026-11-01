//! Pyric Salamander — {1}{R} — Creature — Salamander
//! Oracle: {R}: This creature gets +1/+0 until end of turn. Sacrifice this creature at the beginning of the next end step.
//! Set: MIR #187 — Mirage | Scryfall ID: 7f2dc1a7-4b70-4643-90a8-fdc7877c01ca | Oracle ID: 745a2815-8487-4b78-93b7-d1d4ef6adc8f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PYRIC_SALAMANDER,
    oracle_id = "745a2815-8487-4b78-93b7-d1d4ef6adc8f",
    scryfall_id = "7f2dc1a7-4b70-4643-90a8-fdc7877c01ca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pyric Salamander",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SALAMANDER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[
            Effect::ModifySource {
                modifier: Modifier::ModifyPT(1, 0),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeSelf],
            },
        ],
    )],
);
