//! Larval Scoutlander — {2}{G} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, you may sacrifice a land or Lander. If you do, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Set: EOE #194 — Edge of Eternities | Scryfall ID: d6083f43-58dc-46fc-aeff-347b1080417b | Oracle ID: b3f3d55c-a3f6-441b-a380-60c0734dc514
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LARVAL_SCOUTLANDER,
    oracle_id = "b3f3d55c-a3f6-441b-a380-60c0734dc514",
    scryfall_id = "d6083f43-58dc-46fc-aeff-347b1080417b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Larval Scoutlander",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
