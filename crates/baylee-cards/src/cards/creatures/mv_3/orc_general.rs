//! Orc General — {2}{R} — Creature — Orc Warrior
//! Oracle: {T}, Sacrifice another Orc or Goblin: Other Orc creatures get +1/+1 until end of turn.
//! Set: ME2 #137 — Masters Edition II | Scryfall ID: f09114b4-f98a-4a1f-a0f3-fe2a78b58989 | Oracle ID: 1f696446-30ba-42ab-b4fc-ee9c956b0a62

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ANOTHER_ORC_OR_GOBLIN: Filter = Filter::And(&[
    Filter::Another,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::ORC),
        Filter::HasSubtype(subtypes::creature::GOBLIN),
    ]),
]);

const OTHER_ORC_CREATURES: Filter = Filter::And(&[
    Filter::Another,
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::ORC),
]);

card!(
    index = index::ORC_GENERAL,
    oracle_id = "1f696446-30ba-42ab-b4fc-ee9c956b0a62",
    scryfall_id = "f09114b4-f98a-4a1f-a0f3-fe2a78b58989",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orc General",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(2),
        abilities = &[activated!(
            cost!("", TapSelf, Sacrifice(&ANOTHER_ORC_OR_GOBLIN)),
            &[Effect::CreateContinuousEffect {
                layer: Layer::PtModify,
                filter: &OTHER_ORC_CREATURES,
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn,
            }],
        )],
    ),],
);
