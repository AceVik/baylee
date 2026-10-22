//! Segmented Wurm — {3}{R}{G} — Creature — Wurm
//! Oracle: Whenever this creature becomes the target of a spell or ability, put a -1/-1 counter on it.
//! Set: TMP #269 — Tempest | Scryfall ID: 18be7cfa-bf75-407d-b79f-2fec4b1aacf5 | Oracle ID: eb1baf72-4ef6-4967-8a22-2022c633f3d3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEGMENTED_WURM,
    oracle_id = "eb1baf72-4ef6-4967-8a22-2022c633f3d3",
    scryfall_id = "18be7cfa-bf75-407d-b79f-2fec4b1aacf5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Segmented Wurm",
        mana_cost = mana!("{3}{R}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::AddCounter {
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
    )],
);
