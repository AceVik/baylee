//! Jihad — {W}{W}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a color and an opponent.
//! Oracle: White creatures get +2/+1 as long as the chosen player controls a nontoken permanent of the chosen color.
//! Oracle: When the chosen player controls no nontoken permanents of the chosen color, sacrifice this enchantment.
//! Set: ARN #5 — Arabian Nights | Scryfall ID: b6c7705a-2987-4ef1-92b1-2c55d989ec6f | Oracle ID: b18b9869-8490-4875-a5bb-484c3299f2c5
// IMPLEMENTED — chooses a color and an opponent as it enters; white creatures
// get +2/+1 while that opponent controls a nontoken permanent of that color,
// and it is sacrificed when they control none.

use baylee_cards_dsl::prelude::*;

// "a nontoken permanent of the chosen color"
const NONTOKEN_OF_CHOSEN: Filter =
    Filter::And(&[Filter::Not(&Filter::IsToken), Filter::OfSourceChosenColor]);

// "White creatures"
const WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::JIHAD,
    oracle_id = "b18b9869-8490-4875-a5bb-484c3299f2c5",
    scryfall_id = "b6c7705a-2987-4ef1-92b1-2c55d989ec6f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Jihad",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseColor, EnterModifier::ChooseOpponent],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            WHITE_CREATURE,
            Modifier::ModifyPT(2, 1),
            condition = Some(Condition::ChosenOpponentControls(&NONTOKEN_OF_CHOSEN))
        ),
        triggered!(
            Trigger::State(&Condition::Not(&Condition::ChosenOpponentControls(
                &NONTOKEN_OF_CHOSEN
            ))),
            &[Effect::SacrificeSelf]
        ),
    ],
);
