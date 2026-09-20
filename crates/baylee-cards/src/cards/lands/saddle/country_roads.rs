//! Country Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #253 — Aetherdrift | Scryfall ID: 897acd91-12ba-4fa8-a26e-c09f009167a8 | Oracle ID: c5a39f76-dd1b-442c-9f52-08561ecb91ad
// PARTIAL — the conditional entry and the {W} mana ability are built; the
// sacrifice ability is dropped, see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{artifact, creature};

/// "a Mount or Vehicle you control" — the filter the entry condition asks
/// about, control clause included, the way the battle lands carry
/// `YOUR_BASIC_LAND`.
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
    coverage = Coverage::Partial(
        "the {1}{W} sacrifice ability is missing: its Pilot token's granted \
         ability (\"saddles Mounts and crews Vehicles as though its power \
         were 2 greater\") has no variant, and saddle/crew are not keywords \
         the engine reads"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: "{1}{W}, {T}, Sacrifice this land: Create a 1/1
        // colorless Pilot creature token with 'This token saddles Mounts and
        // crews Vehicles as though its power were 2 greater.' Activate only
        // as a sorcery." — no `Modifier`, keyword or effect says "taps as
        // though its power were 2 greater", so the token would arrive as a
        // 1/1 with no rules text and the ability would be a wrong card
        // rather than a missing one.
    ],
);
