//! Tireless Hauler // Dire-Strain Brawler — {4}{G} — Creature — Human Werewolf // Creature — Werewolf
//! Set: MID #203 — Innistrad: Midnight Hunt | Scryfall ID: 3e96f9a6-c215-42b1-aa02-8e6143fe5bd7 | Oracle ID: c31e9db3-5d9d-470a-871a-b4b5b0536db5
//! Face: Tireless Hauler — {4}{G} — Creature — Human Werewolf
//! Face: Dire-Strain Brawler —  — Creature — Werewolf
//! Oracle: Tireless Hauler — Vigilance. Daybound.
//! Oracle: Dire-Strain Brawler — Vigilance. Nightbound.
// IMPLEMENTED — vigilance on both faces, daybound on the front and
// nightbound on the back (CR 702.145a).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1364,
    oracle_id: "c31e9db3-5d9d-470a-871a-b4b5b0536db5",
    scryfall_id: "3e96f9a6-c215-42b1-aa02-8e6143fe5bd7",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Tireless Hauler",
        mana_cost: baylee_core::mana!("{4}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::WEREWOLF],
        power: Some(4),
        toughness: Some(5),
        keywords: KeywordSet::VIGILANCE.union(KeywordSet::DAYBOUND),
    },
    face! {
        name: "Dire-Strain Brawler",
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::WEREWOLF],
        power: Some(6),
        toughness: Some(6),
        castable_from_hand: false,
        keywords: KeywordSet::VIGILANCE.union(KeywordSet::NIGHTBOUND),
        color_indicator: ColorSet::from_slice(&[Color::Green]),
    },
    ],
    coverage: Coverage::Implemented,
}
