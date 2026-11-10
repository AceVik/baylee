//! Wild Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {G}.
//! Oracle: {1}{G}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #269 — Aetherdrift | Scryfall ID: 4d3d48d1-a98e-40af-b04c-c40b9d52e9ee | Oracle ID: 36fbc8ba-bb4c-4e5e-9031-78c36e376851

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MOUNT_OR_VEHICLE: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::MOUNT),
        Filter::HasSubtype(subtypes::artifact::VEHICLE),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::WILD_ROADS,
    oracle_id = "36fbc8ba-bb4c-4e5e-9031-78c36e376851",
    scryfall_id = "4d3d48d1-a98e-40af-b04c-c40b9d52e9ee",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wild Roads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&MOUNT_OR_VEHICLE)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{1}{G}", TapSelf, SacrificeSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::PILOT_1_1_COLORLESS
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
