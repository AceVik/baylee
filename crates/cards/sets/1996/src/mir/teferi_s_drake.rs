//! Teferi's Drake — {2}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: MIR #97 — Mirage | Scryfall ID: c24823df-5651-4578-a0c8-f9f52f66abe4 | Oracle ID: 3605c5ac-3b35-41a5-9266-286fb1d11716
// IMPLEMENTED — 3/2 Drake with flying and phasing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEFERI_S_DRAKE,
    oracle_id = "3605c5ac-3b35-41a5-9266-286fb1d11716",
    scryfall_id = "c24823df-5651-4578-a0c8-f9f52f66abe4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING.union(KeywordSet::PHASING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Teferi's Drake",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(3),
        toughness = Some(2),
        keywords = KeywordSet::FLYING.union(KeywordSet::PHASING),
    ),],
);
