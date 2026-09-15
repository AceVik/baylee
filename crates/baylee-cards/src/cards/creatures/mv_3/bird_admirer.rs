//! Bird Admirer // Wing Shredder — {2}{G} — Creature — Human Archer Werewolf // Creature — Werewolf
//! Oracle: Reach
//! Oracle: Daybound (If a player casts no spells during their own turn, it becomes night next turn.)
//! Oracle: Reach
//! Oracle: Nightbound (If a player casts at least two spells during their own turn, it becomes day next turn.)
//! Set: MID #169 — Innistrad: Midnight Hunt | Scryfall ID: 71ccc444-54c8-4f7c-a425-82bc3eea1eb0 | Oracle ID: 58bd02ae-2676-4c9c-b24e-2bd51be8bde7
//! Face: Bird Admirer — {2}{G} — Creature — Human Archer Werewolf
//! Face: Wing Shredder —  — Creature — Werewolf
// IMPLEMENTED — one keyword on each face plus the pair that turns the card
// over. Reach is printed on both halves, which is why it is written twice:
// a back face inherits nothing (CR 702.145a puts daybound and nightbound on
// opposite faces, so the two keyword sets have to be able to disagree).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = 1360,
    oracle_id = "58bd02ae-2676-4c9c-b24e-2bd51be8bde7",
    scryfall_id = "71ccc444-54c8-4f7c-a425-82bc3eea1eb0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Bird Admirer",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::CREATURE,
            subtypes = &[
                subtypes::creature::HUMAN,
                subtypes::creature::ARCHER,
                subtypes::creature::WEREWOLF
            ],
            power = Some(1),
            toughness = Some(4),
            keywords = KeywordSet::REACH.union(KeywordSet::DAYBOUND),
        ),
        face!(
            name = "Wing Shredder",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEREWOLF],
            power = Some(3),
            toughness = Some(5),
            castable_from_hand = false,
            keywords = KeywordSet::REACH.union(KeywordSet::NIGHTBOUND),
            color_indicator = ColorSet::from_slice(&[Color::Green]),
        ),
    ],
    coverage = Coverage::Implemented,
);
