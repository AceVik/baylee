//! Suq'Ata Lancer — {2}{R} — Creature — Human Knight
//! Oracle: Haste
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: DMR #145 — Dominaria Remastered | Scryfall ID: c9cd519a-c6f4-47a8-b773-a058e50e4440 | Oracle ID: f348b326-8a73-4df9-8faf-e271a36ce373
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUQ_ATA_LANCER,
    oracle_id = "f348b326-8a73-4df9-8faf-e271a36ce373",
    scryfall_id = "c9cd519a-c6f4-47a8-b773-a058e50e4440",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE.union(KeywordSet::FLANKING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Suq'Ata Lancer",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::HASTE.union(KeywordSet::FLANKING),
    ),],
);
