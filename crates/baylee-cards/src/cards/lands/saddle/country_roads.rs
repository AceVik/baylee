//! Country Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #253 — Aetherdrift | Scryfall ID: 897acd91-12ba-4fa8-a26e-c09f009167a8 | Oracle ID: c5a39f76-dd1b-442c-9f52-08561ecb91ad

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{artifact, creature};

static MOUNT_OR_VEHICLE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(creature::MOUNT),
        Filter::HasSubtype(artifact::VEHICLE),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::COUNTRY_ROADS,
    oracle_id = "c5a39f76-dd1b-442c-9f52-08561ecb91ad",
    scryfall_id = "897acd91-12ba-4fa8-a26e-c09f009167a8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Country Roads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&MOUNT_OR_VEHICLE_YOU_CONTROL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{W}", TapSelf, SacrificeSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::PILOT_1_1_COLORLESS
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
