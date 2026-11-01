//! Dream Prowler — {2}{U}{U} — Creature — Illusion
//! Oracle: This creature can't be blocked as long as it's attacking alone.
//! Set: TPR #47 — Tempest Remastered | Scryfall ID: 08d90394-22fe-42ae-83ba-9cf88cfe1d1d | Oracle ID: b56c5b39-2465-4a25-b806-bb84866e576f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DREAM_PROWLER,
    oracle_id = "b56c5b39-2465-4a25-b806-bb84866e576f",
    scryfall_id = "08d90394-22fe-42ae-83ba-9cf88cfe1d1d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dream Prowler",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(1),
        toughness = Some(5),
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::This, Filter::Attacking]),
        Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
        condition = Some(Condition::BattlefieldCountAtMost(
            &Filter::ATTACKING_CREATURE,
            1
        )),
    )],
);
