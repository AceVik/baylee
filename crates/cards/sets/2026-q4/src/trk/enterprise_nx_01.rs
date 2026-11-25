//! Enterprise NX-01 — {3}{U} — Legendary Artifact — Spacecraft
//! Oracle: When Enterprise NX-01 enters, tap target creature an opponent controls and put a stun counter on it. (If a permanent with a stun counter would become untapped, remove one from it instead.)
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying, vigilance
//! Set: TRK #58 — Star Trek | Scryfall ID: 385638fe-cafb-4541-ba37-19bc57520c48 | Oracle ID: 947729a8-f8b0-4e08-9a87-6602af0e3c0a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENTERPRISE_NX_01,
    oracle_id = "947729a8-f8b0-4e08-9a87-6602af0e3c0a",
    scryfall_id = "385638fe-cafb-4541-ba37-19bc57520c48",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Enterprise NX-01",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
