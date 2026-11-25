//! Shuttlecraft Galileo — {3}{W} — Legendary Artifact — Spacecraft
//! Oracle: Landfall — Whenever a land you control enters, create a 1/1 red Officer creature token.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying
//! Oracle: Whenever Shuttlecraft Galileo attacks, up to seven target attacking creatures gain flying until end of turn.
//! Set: TRC #28 — Star Trek Commander | Scryfall ID: a4242f45-75c1-4b23-9174-bd0fe70fa039 | Oracle ID: 12e4e6b6-e684-49df-9373-d1e057664272
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHUTTLECRAFT_GALILEO,
    oracle_id = "12e4e6b6-e684-49df-9373-d1e057664272",
    scryfall_id = "a4242f45-75c1-4b23-9174-bd0fe70fa039",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Shuttlecraft Galileo",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
