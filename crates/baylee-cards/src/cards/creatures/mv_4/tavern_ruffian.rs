//! Tavern Ruffian // Tavern Smasher — {3}{R} — Creature — Human Warrior Werewolf // Creature — Werewolf
//! Oracle: Daybound (If a player casts no spells during their own turn, it becomes night next turn.)
//! Oracle: Nightbound (If a player casts at least two spells during their own turn, it becomes day next turn.)
//! Set: MID #163 — Innistrad: Midnight Hunt | Scryfall ID: 1d7b2d05-ce5c-4b73-8fa6-d9b69619d58c | Oracle ID: 73a3b9a1-37a0-469a-9557-8c118a1ee78f
//! Face: Tavern Ruffian — {3}{R} — Creature — Human Warrior Werewolf
//! Face: Tavern Smasher —  — Creature — Werewolf
// IMPLEMENTED — the whole card is the pair that turns it over, which makes
// it the honest test of the mechanic: a 2/5 by day and a 6/5 by night, with
// nothing else printed on either half to explain a difference in play.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAVERN_RUFFIAN,
    oracle_id = "73a3b9a1-37a0-469a-9557-8c118a1ee78f",
    scryfall_id = "1d7b2d05-ce5c-4b73-8fa6-d9b69619d58c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Tavern Ruffian",
            mana_cost = mana!("{3}{R}"),
            types = TypeSet::CREATURE,
            subtypes = &[
                subtypes::creature::HUMAN,
                subtypes::creature::WARRIOR,
                subtypes::creature::WEREWOLF
            ],
            power = Some(2),
            toughness = Some(5),
            keywords = KeywordSet::DAYBOUND,
        ),
        face!(
            name = "Tavern Smasher",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEREWOLF],
            power = Some(6),
            toughness = Some(5),
            castable_from_hand = false,
            keywords = KeywordSet::NIGHTBOUND,
            color_indicator = ColorSet::from_slice(&[Color::Red]),
        ),
    ],
    coverage = Coverage::Implemented,
);
