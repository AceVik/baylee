//! Goblin Elite Infantry — {1}{R} — Creature — Goblin Warrior
//! Oracle: Whenever this creature blocks or becomes blocked, it gets -1/-1 until end of turn.
//! Set: 10E #206 — Tenth Edition | Scryfall ID: aa052041-14db-4a3f-86ad-683c1f162cd7 | Oracle ID: a6a8861c-b6fe-45a6-86e4-9160c36362ff
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_ELITE_INFANTRY,
    oracle_id = "a6a8861c-b6fe-45a6-86e4-9160c36362ff",
    scryfall_id = "aa052041-14db-4a3f-86ad-683c1f162cd7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Elite Infantry",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(-1, -1),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
