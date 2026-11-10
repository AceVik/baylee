//! Shadow Rider — {2}{B}{B} — Creature — Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: WTH #79 — Weatherlight | Scryfall ID: 5bfdec24-e689-4cca-a546-a8f5d0929f8d | Oracle ID: a518b31a-7e9a-460d-839f-39cb1de4f570
// IMPLEMENTED — Flanking: the keyword alone, the engine raises the -1/-1 trigger (CR 702.25a).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHADOW_RIDER,
    oracle_id = "a518b31a-7e9a-460d-839f-39cb1de4f570",
    scryfall_id = "5bfdec24-e689-4cca-a546-a8f5d0929f8d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shadow Rider",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KNIGHT],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLANKING,
    ),],
);
