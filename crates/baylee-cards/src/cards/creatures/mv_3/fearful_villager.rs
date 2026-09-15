//! Fearful Villager // Fearsome Werewolf — {2}{R} — Creature — Human Werewolf // Creature — Werewolf
//! Oracle: Menace (This creature can't be blocked except by two or more creatures.)
//! Oracle: Daybound (If a player casts no spells during their own turn, it becomes night next turn.)
//! Oracle: Menace (This creature can't be blocked except by two or more creatures.)
//! Oracle: Nightbound (If a player casts at least two spells during their own turn, it becomes day next turn.)
//! Set: VOW #157 — Innistrad: Crimson Vow | Scryfall ID: 5eb3a08e-1d31-4ab9-854f-a86b060696ec | Oracle ID: 5fd09dbc-8bcd-4fe0-91b5-b00e721fa7eb
//! Face: Fearful Villager — {2}{R} — Creature — Human Werewolf
//! Face: Fearsome Werewolf —  — Creature — Werewolf
// IMPLEMENTED — menace on both faces, daybound on the front and nightbound
// on the back (CR 702.145a).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = 22446,
    oracle_id = "5fd09dbc-8bcd-4fe0-91b5-b00e721fa7eb",
    scryfall_id = "5eb3a08e-1d31-4ab9-854f-a86b060696ec",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Fearful Villager",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WEREWOLF],
            power = Some(2),
            toughness = Some(3),
            keywords = KeywordSet::MENACE.union(KeywordSet::DAYBOUND),
        ),
        face!(
            name = "Fearsome Werewolf",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEREWOLF],
            power = Some(4),
            toughness = Some(3),
            castable_from_hand = false,
            keywords = KeywordSet::MENACE.union(KeywordSet::NIGHTBOUND),
            color_indicator = ColorSet::from_slice(&[Color::Red]),
        ),
    ],
    coverage = Coverage::Implemented,
);
