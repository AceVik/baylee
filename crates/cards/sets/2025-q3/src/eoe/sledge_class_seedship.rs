//! Sledge-Class Seedship — {2}{G} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Oracle: Whenever this Spacecraft attacks, you may put a creature card from your hand onto the battlefield.
//! Set: EOE #208 — Edge of Eternities | Scryfall ID: dc0cb4b6-cc20-49ea-84b2-2f3af3b0a19e | Oracle ID: bcd086ce-de5c-4b34-92a1-ece22055e479
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SLEDGE_CLASS_SEEDSHIP,
    oracle_id = "bcd086ce-de5c-4b34-92a1-ece22055e479",
    scryfall_id = "dc0cb4b6-cc20-49ea-84b2-2f3af3b0a19e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sledge-Class Seedship",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
