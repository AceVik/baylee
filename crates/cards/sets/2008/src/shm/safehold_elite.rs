//! Safehold Elite — {1}{G/W} — Creature — Elf Scout
//! Oracle: Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)
//! Set: UMA #220 — Ultimate Masters | Scryfall ID: d049f233-7fcb-49cb-897b-a12d57692fe0 | Oracle ID: 68ca91ba-31fb-47e0-9b32-e4f3504cbbca

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAFEHOLD_ELITE,
    oracle_id = "68ca91ba-31fb-47e0-9b32-e4f3504cbbca",
    scryfall_id = "d049f233-7fcb-49cb-897b-a12d57692fe0",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Safehold Elite",
        mana_cost = mana!("{1}{G/W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SCOUT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::PERSIST,
    ),],
);
