//! Mtenda Herder — {W} — Creature — Human Scout
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: MIR #29 — Mirage | Scryfall ID: 51f30a3d-1421-4706-b17f-39a9ec7a0d8b | Oracle ID: 0edf5bac-36a9-4f35-afd1-73f7e1aab51b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MTENDA_HERDER,
    oracle_id = "0edf5bac-36a9-4f35-afd1-73f7e1aab51b",
    scryfall_id = "51f30a3d-1421-4706-b17f-39a9ec7a0d8b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mtenda Herder",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SCOUT],
        power = Some(1),
        toughness = Some(1),
        keywords = KeywordSet::FLANKING,
    ),],
);
