//! Mogg Squad — {1}{R} — Creature — Goblin
//! Oracle: This creature gets -1/-1 for each other creature on the battlefield.
//! Set: TMP #192 — Tempest | Scryfall ID: 4b267071-42a6-4e25-9c92-5bca32f8d9af | Oracle ID: 4dac142a-c8f8-45c8-85d3-40e170dee112
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Another]);

card!(
    index = index::MOGG_SQUAD,
    oracle_id = "4dac142a-c8f8-45c8-85d3-40e170dee112",
    scryfall_id = "4b267071-42a6-4e25-9c92-5bca32f8d9af",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mogg Squad",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ModifyPTByCount(PtCount::Complement {
            from: 0,
            count: &PtCount::OnBattlefield(&OTHER_CREATURE),
        }),
    )],
);
