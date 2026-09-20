//! Wild Roads — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mount or Vehicle.
//! Oracle: {T}: Add {G}.
//! Oracle: {1}{G}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with "This token saddles Mounts and crews Vehicles as though its power were 2 greater." Activate only as a sorcery.
//! Set: DFT #269 — Aetherdrift | Scryfall ID: 4d3d48d1-a98e-40af-b04c-c40b9d52e9ee | Oracle ID: 36fbc8ba-bb4c-4e5e-9031-78c36e376851
// PARTIAL — entry condition and the {G} mana ability are built; the {1}{G}
// sacrifice ability is not, for the reason written at the ability below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "A Mount or Vehicle you control" — the sentence names a permanent, and
/// whose it is has to be in the filter: an opponent's Mount does not satisfy
/// it. The entering land is neither a Mount nor a Vehicle, so it never counts
/// itself.
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
    coverage = Coverage::Partial(
        "the {1}{G} sacrifice ability creates a Pilot token that `crate::tokens` \
         does not carry, and that token's \"saddles Mounts and crews Vehicles as \
         though its power were 2 greater\" clause has no DSL variant"
    ),
    // NOT SUPPORTED: "{1}{G}, {T}, Sacrifice this land: Create a 1/1 colorless
    // Pilot creature token with 'This token saddles Mounts and crews Vehicles
    // as though its power were 2 greater.' Activate only as a sorcery." — no
    // Pilot token exists in `crate::tokens` (a card file may not define one),
    // and `Modifier` has nothing that raises a creature's power for saddling
    // and crewing. The ability comes off rather than shipping a {1}{G}
    // sacrifice that makes nothing.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
