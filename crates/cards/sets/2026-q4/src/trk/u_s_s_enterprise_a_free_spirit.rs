//! U.S.S. Enterprise-A, Free Spirit — {3}{W}{W} — Legendary Artifact — Spacecraft
//! Oracle: Ward {2}
//! Oracle: When U.S.S. Enterprise-A enters, create three 1/1 red Officer creature tokens.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying, lifelink
//! Set: TRK #43 — Star Trek | Scryfall ID: 99429015-6e5d-4ecf-aaf7-eba181bcffc7 | Oracle ID: 5e2958fd-ede6-4563-babc-4924d3e95754
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_ENTERPRISE_A_FREE_SPIRIT,
    oracle_id = "5e2958fd-ede6-4563-babc-4924d3e95754",
    scryfall_id = "99429015-6e5d-4ecf-aaf7-eba181bcffc7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Enterprise-A, Free Spirit",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
