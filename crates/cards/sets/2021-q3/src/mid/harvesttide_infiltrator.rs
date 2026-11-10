//! Harvesttide Infiltrator // Harvesttide Assailant — {2}{R} — Creature — Human Werewolf // Creature — Werewolf
//! Oracle: Trample
//! Oracle: Daybound (If a player casts no spells during their own turn, it becomes night next turn.)
//! Oracle: Trample
//! Oracle: Nightbound (If a player casts at least two spells during their own turn, it becomes day next turn.)
//! Set: MID #143 — Innistrad: Midnight Hunt | Scryfall ID: 35fdb976-291c-4824-9518-dd8c9f93fcde | Oracle ID: 8669f2e1-3e98-4fa5-ba4f-a0860b92c609
//! Face: Harvesttide Infiltrator — {2}{R} — Creature — Human Werewolf
//! Face: Harvesttide Assailant —  — Creature — Werewolf
// IMPLEMENTED — trample on both faces, daybound on the front and nightbound
// on the back (CR 702.145a).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HARVESTTIDE_INFILTRATOR,
    oracle_id = "8669f2e1-3e98-4fa5-ba4f-a0860b92c609",
    scryfall_id = "35fdb976-291c-4824-9518-dd8c9f93fcde",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Harvesttide Infiltrator",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WEREWOLF],
            power = Some(3),
            toughness = Some(2),
            keywords = KeywordSet::TRAMPLE.union(KeywordSet::DAYBOUND),
        ),
        face!(
            name = "Harvesttide Assailant",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEREWOLF],
            power = Some(4),
            toughness = Some(4),
            castable_from_hand = false,
            keywords = KeywordSet::TRAMPLE.union(KeywordSet::NIGHTBOUND),
            color_indicator = ColorSet::from_slice(&[Color::Red]),
        ),
    ],
    coverage = Coverage::Implemented,
);
