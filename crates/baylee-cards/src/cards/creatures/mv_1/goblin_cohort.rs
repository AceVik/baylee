//! Goblin Cohort — {R} — Creature — Goblin Warrior
//! Oracle: This creature can't attack unless you've cast a creature spell this turn.
//! Set: EVG #38 — Duel Decks Anthology: Elves vs. Goblins | Scryfall ID: faa652d7-16d3-400f-9ba4-7ba8078d4a08 | Oracle ID: 88f9a7e3-0b35-43e4-b16a-19c54a1e4e2c
// IMPLEMENTED — its controller must have cast a creature spell this turn for it to attack.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_COHORT,
    oracle_id = "88f9a7e3-0b35-43e4-b16a-19c54a1e4e2c",
    scryfall_id = "faa652d7-16d3-400f-9ba4-7ba8078d4a08",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Cohort",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantAttack,
        condition = Some(Condition::Not(&Condition::CastCreatureThisTurn))
    )],
);
