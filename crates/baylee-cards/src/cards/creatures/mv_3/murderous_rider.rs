//! Murderous Rider // Swift End — {1}{B}{B} — Creature — Zombie Knight // Instant — Adventure
//! Oracle: Lifelink
//! Oracle: When this creature dies, put it on the bottom of its owner's library.
//! Oracle: Destroy target creature or planeswalker. You lose 2 life. (Then exile this card. You may cast the creature later from exile.)
//! Set: MOC #258 — March of the Machine Commander | Scryfall ID: 80fffad3-2486-4350-8dff-54a215ebfc28 | Oracle ID: 1080c5b5-6651-4c6a-93e6-099fbe389e26
//! Face: Murderous Rider — {1}{B}{B} — Creature — Zombie Knight
//! Face: Swift End — {1}{B}{B} — Instant — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MURDEROUS_RIDER,
    oracle_id = "1080c5b5-6651-4c6a-93e6-099fbe389e26",
    scryfall_id = "80fffad3-2486-4350-8dff-54a215ebfc28",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Murderous Rider",
            mana_cost = mana!("{1}{B}{B}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::KNIGHT],
            power = Some(2),
            toughness = Some(3),
        ),
        face!(
            name = "Swift End",
            mana_cost = mana!("{1}{B}{B}"),
            types = TypeSet::INSTANT,
            subtypes = &[subtypes::spell::ADVENTURE],
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
