//! Jolrael's Centaur — {1}{G}{G} — Creature — Centaur Archer
//! Oracle: Shroud (This creature can't be the target of spells or abilities.)
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: MIR #222 — Mirage | Scryfall ID: 31588ea3-31d2-4118-9b9f-4ce820c16a15 | Oracle ID: 39b699e1-79c6-4960-97dc-d58574181bbe
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JOLRAEL_S_CENTAUR,
    oracle_id = "39b699e1-79c6-4960-97dc-d58574181bbe",
    scryfall_id = "31588ea3-31d2-4118-9b9f-4ce820c16a15",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jolrael's Centaur",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR, subtypes::creature::ARCHER],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::SHROUD.union(KeywordSet::FLANKING),
    ),],
);
