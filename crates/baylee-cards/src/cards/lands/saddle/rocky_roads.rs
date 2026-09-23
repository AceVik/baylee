//! Rocky Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {R}.
//! Oracle: {1}{R}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #261 — Aetherdrift | Scryfall ID: 6f0d4e8a-aab5-458e-bc0e-2b6b0054646a | Oracle ID: a659c29f-aaca-44c5-8426-cdafcb195f86
// PARTIAL — the entry clause (tapped unless you control a Mount or Vehicle)
// and {T}: Add {R} are built; the sacrifice ability is left off the card.

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
    index = index::ROCKY_ROADS,
    oracle_id = "a659c29f-aaca-44c5-8426-cdafcb195f86",
    scryfall_id = "6f0d4e8a-aab5-458e-bc0e-2b6b0054646a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rocky Roads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&MOUNT_OR_VEHICLE_YOU_CONTROL)],
    )],
    coverage = Coverage::Partial(
        "the Pilot token's \"saddles Mounts and crews Vehicles as though its power were 2 greater\": the DSL has no saddling or crewing vocabulary, so the clause the token is created with cannot be said",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // NOT SUPPORTED: {1}{R}, {T}, Sacrifice this land: Create a 1/1
        // colorless Pilot creature token with "This token saddles Mounts and
        // crews Vehicles as though its power were 2 greater." Activate only as
        // a sorcery. — saddling a Mount and crewing a Vehicle exist nowhere in
        // this DSL, so the clause the token is defined with is unsayable and
        // the whole ability is dropped rather than played with a 1/1 that
        // silently loses its text.
    ],
);
