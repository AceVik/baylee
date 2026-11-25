//! Dawnsire, Sunstar Dreadnought — {5} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 20+.)
//! Oracle: 10+ | Whenever you attack, Dawnsire deals 100 damage to up to one target creature or planeswalker.
//! Oracle: 20+ | Flying
//! Set: EOE #238 — Edge of Eternities | Scryfall ID: 6133355c-3dcf-466a-b771-fe6c44d4fa4d | Oracle ID: afc9436b-8cad-4916-929d-ff33a37b42d5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DAWNSIRE_SUNSTAR_DREADNOUGHT,
    oracle_id = "afc9436b-8cad-4916-929d-ff33a37b42d5",
    scryfall_id = "6133355c-3dcf-466a-b771-fe6c44d4fa4d",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Dawnsire, Sunstar Dreadnought",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(20),
        toughness = Some(20),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
