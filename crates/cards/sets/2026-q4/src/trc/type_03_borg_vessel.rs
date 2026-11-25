//! Type-03 Borg Vessel — {3}{W} — Artifact — Spacecraft
//! Oracle: Artifact spells you cast cost {1} less to cast.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Oracle: Whenever you cast a spell, create a 1/1 colorless Borg artifact creature token.
//! Set: TRC #33 — Star Trek Commander | Scryfall ID: c91ec9e3-abd5-4c23-b342-0c6a12cc970a | Oracle ID: ff2ab17e-185b-4782-a4e2-e8ace40ef1bb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TYPE_03_BORG_VESSEL,
    oracle_id = "ff2ab17e-185b-4782-a4e2-e8ace40ef1bb",
    scryfall_id = "c91ec9e3-abd5-4c23-b342-0c6a12cc970a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Type-03 Borg Vessel",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
