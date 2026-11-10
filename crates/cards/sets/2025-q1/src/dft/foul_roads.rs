//! Foul Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #255 — Aetherdrift | Scryfall ID: a37c026a-c89f-41f3-8812-424124dd3760 | Oracle ID: d4e4c8a5-e97b-4295-a403-d17834f73502

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{artifact, creature};

card!(
    index = index::FOUL_ROADS,
    oracle_id = "d4e4c8a5-e97b-4295-a403-d17834f73502",
    scryfall_id = "a37c026a-c89f-41f3-8812-424124dd3760",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Foul Roads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::And(&[
            Filter::Or(&[
                Filter::HasSubtype(creature::MOUNT),
                Filter::HasSubtype(artifact::VEHICLE),
            ]),
            Filter::ControlledByYou,
        ]))],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{1}{B}", TapSelf, SacrificeSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::PILOT_1_1_COLORLESS
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
