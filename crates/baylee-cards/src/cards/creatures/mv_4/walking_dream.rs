//! Walking Dream — {3}{U} — Creature — Illusion
//! Oracle: This creature can't be blocked.
//! Oracle: This creature doesn't untap during your untap step if an opponent controls two or more creatures.
//! Set: STH #49 — Stronghold | Scryfall ID: bd2f906d-b127-400a-809b-449f1e7f647b | Oracle ID: 286e7ef1-d763-4b48-8603-a161dcf7efbc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALKING_DREAM,
    oracle_id = "286e7ef1-d763-4b48-8603-a161dcf7efbc",
    scryfall_id = "bd2f906d-b127-400a-809b-449f1e7f647b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Walking Dream",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(3),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::UNBLOCKABLE,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::DoesNotUntap,
        condition = Some(Condition::OpponentControlCount(
            &Filter::OPPONENT_CREATURE,
            2
        )),
    )],
);
