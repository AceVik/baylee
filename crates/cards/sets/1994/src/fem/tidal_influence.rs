//! Tidal Influence — {2}{U} — Enchantment
//! Oracle: Cast this spell only if no permanents named Tidal Influence are on the battlefield.
//! Oracle: This enchantment enters with a tide counter on it.
//! Oracle: At the beginning of your upkeep, put a tide counter on this enchantment.
//! Oracle: As long as there is exactly one tide counter on this enchantment, all blue creatures get -2/-0.
//! Oracle: As long as there are exactly three tide counters on this enchantment, all blue creatures get +2/+0.
//! Oracle: Whenever there are four or more tide counters on this enchantment, remove all tide counters from it.
//! Set: FEM #28 — Fallen Empires | Scryfall ID: b2192c7b-ef6f-4ff6-9017-b1a125340517 | Oracle ID: b1393388-b8b3-4c54-a086-f5e6d9908972
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);
const NAMED_TIDAL_INFLUENCE: Filter = Filter::Named("Tidal Influence");
const FOUR_OR_MORE_TIDE: Condition = Condition::CountersOnSelf(counters::TIDE, 4);

card!(
    index = index::TIDAL_INFLUENCE,
    oracle_id = "b1393388-b8b3-4c54-a086-f5e6d9908972",
    scryfall_id = "b2192c7b-ef6f-4ff6-9017-b1a125340517",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tidal Influence",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::TIDE,
            amount: Amount::Fixed(1),
        }],
    ),],
    abilities = &[
        spell!(
            &[],
            condition = Some(Condition::BattlefieldCountAtMost(&NAMED_TIDAL_INFLUENCE, 0)),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: counters::TIDE,
                amount: Amount::Fixed(1),
            }],
        ),
        static_ability!(
            BLUE_CREATURE,
            Modifier::ModifyPT(-2, 0),
            condition = Some(Condition::CountersOnSelfExactly(counters::TIDE, 1)),
        ),
        static_ability!(
            BLUE_CREATURE,
            Modifier::ModifyPT(2, 0),
            condition = Some(Condition::CountersOnSelfExactly(counters::TIDE, 3)),
        ),
        triggered!(
            Trigger::State(&FOUR_OR_MORE_TIDE),
            &[Effect::RemoveAllCountersSelf {
                kind: counters::TIDE,
            }],
        ),
    ],
);
