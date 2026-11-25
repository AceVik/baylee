//! Transwarp Hub — {2}{U} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, mill five cards. You may put up to two artifact cards from among them into your hand.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery.)
//! Oracle: 3+ | {T}: Target creature can't be blocked this turn.
//! Set: TRC #52 — Star Trek Commander | Scryfall ID: 60bcc801-7c0a-449f-a06d-d8b96c1cb42f | Oracle ID: 4325d4d2-5892-4c20-a994-7396b715e94f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRANSWARP_HUB,
    oracle_id = "4325d4d2-5892-4c20-a994-7396b715e94f",
    scryfall_id = "60bcc801-7c0a-449f-a06d-d8b96c1cb42f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Transwarp Hub",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
