//! Rocky Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {R}.
//! Oracle: {1}{R}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #261 — Aetherdrift | Scryfall ID: 6f0d4e8a-aab5-458e-bc0e-2b6b0054646a | Oracle ID: a659c29f-aaca-44c5-8426-cdafcb195f86

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
    index = index::ROCKY_ROADS,
    oracle_id = "a659c29f-aaca-44c5-8426-cdafcb195f86",
    scryfall_id = "6f0d4e8a-aab5-458e-bc0e-2b6b0054646a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rocky Roads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&MOUNT_OR_VEHICLE_YOU_CONTROL)],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{1}{R}", TapSelf, SacrificeSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::PILOT_1_1_COLORLESS
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
