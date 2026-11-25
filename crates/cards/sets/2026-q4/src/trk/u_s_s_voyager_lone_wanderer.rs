//! U.S.S. Voyager, Lone Wanderer — {3}{G} — Legendary Artifact — Spacecraft
//! Oracle: When U.S.S. Voyager enters, reveal cards from the top of your library until you reveal two land cards. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying, vigilance
//! Set: TRK #222 — Star Trek | Scryfall ID: aaf8a0b2-c6cb-41cb-913d-7ae1d75e555d | Oracle ID: 33e8904a-9285-4031-900c-0a5c8f0bf6c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_VOYAGER_LONE_WANDERER,
    oracle_id = "33e8904a-9285-4031-900c-0a5c8f0bf6c0",
    scryfall_id = "aaf8a0b2-c6cb-41cb-913d-7ae1d75e555d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Voyager, Lone Wanderer",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
