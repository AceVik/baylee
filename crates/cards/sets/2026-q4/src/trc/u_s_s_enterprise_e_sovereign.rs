//! U.S.S. Enterprise-E, Sovereign — {3}{W}{W} — Legendary Artifact — Spacecraft
//! Oracle: When U.S.S. Enterprise-E enters, destroy all tapped creatures.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying, vigilance
//! Set: TRC #34 — Star Trek Commander | Scryfall ID: 15e74da6-477c-4ab3-8b91-40f4b2d22f94 | Oracle ID: fa667031-25cf-45c4-97ff-19a73cb30018
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_ENTERPRISE_E_SOVEREIGN,
    oracle_id = "fa667031-25cf-45c4-97ff-19a73cb30018",
    scryfall_id = "15e74da6-477c-4ab3-8b91-40f4b2d22f94",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Enterprise-E, Sovereign",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(7),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
