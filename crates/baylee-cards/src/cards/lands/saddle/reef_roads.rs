//! Reef Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {U}.
//! Oracle: {1}{U}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #259 — Aetherdrift | Scryfall ID: 73a8171a-2629-4356-ae86-b4d03aa14bd3 | Oracle ID: 32438050-5ae7-4c19-bcaf-5a07a673e0e0
// PARTIAL — the tapped-unless-Mount/Vehicle entry (EnterModifier::TappedUnless)
// and {T}: Add {U} are built; the sacrifice ability is not, see NOT SUPPORTED.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{artifact, creature};

/// "a Mount or Vehicle you control" — control clause included, because
/// the entry check walks the whole battlefield and scopes nothing itself.
static MOUNT_OR_VEHICLE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(creature::MOUNT),
        Filter::HasSubtype(artifact::VEHICLE),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::REEF_ROADS,
    oracle_id = "32438050-5ae7-4c19-bcaf-5a07a673e0e0",
    scryfall_id = "73a8171a-2629-4356-ae86-b4d03aa14bd3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the {1}{U}, {T}, Sacrifice ability is off the card: its Pilot token saddles Mounts \
         and crews Vehicles as though its power were 2 greater, and no Modifier can say that"
    ),
    faces = &[face!(
        name = "Reef Roads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&MOUNT_OR_VEHICLE_YOU_CONTROL)],
    )],
    // NOT SUPPORTED: "{1}{U}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature
    // token with 'This token saddles Mounts and crews Vehicles as though its power were 2
    // greater.' Activate only as a sorcery." — the token's granted ability is a rule about
    // saddling and crewing, which has no Modifier variant, so the whole ability is dropped.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
