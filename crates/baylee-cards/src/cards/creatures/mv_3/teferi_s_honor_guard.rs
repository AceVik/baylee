//! Teferi's Honor Guard — {2}{W} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: {U}{U}: This creature phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: VIS #22 — Visions | Scryfall ID: 4177d5bf-db48-4bbf-bbd4-ee6313031920 | Oracle ID: 8a3a8a02-b7da-4648-a772-c9efb7c332da
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEFERI_S_HONOR_GUARD,
    oracle_id = "8a3a8a02-b7da-4648-a772-c9efb7c332da",
    scryfall_id = "4177d5bf-db48-4bbf-bbd4-ee6313031920",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Teferi's Honor Guard",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    // Flanking is the keyword above: the engine raises the -1/-1 trigger itself (CR 702.25a).
    abilities = &[activated!(
        cost!("{U}{U}"),
        &[Effect::PhaseOut { target: None }]
    )],
);
