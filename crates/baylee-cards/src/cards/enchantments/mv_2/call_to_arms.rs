//! Call to Arms — {1}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a color and an opponent.
//! Oracle: White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common.
//! Oracle: When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment.
//! Set: ME3 #4 — Masters Edition III | Scryfall ID: 1a772cac-c853-4621-a61f-a9b06f2e2dec | Oracle ID: 72eaaa73-3263-46e9-aa86-5862d3aed20a
// PARTIAL: the anthem is unconditional and the sacrifice trigger is missing; the color and opponent are chosen but read by nothing.

use baylee_cards_dsl::prelude::*;

const WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

// NOT SUPPORTED: "as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common". No condition counts a player's permanents by color, so white creatures get +1/+1 whatever that player controls.
// NOT SUPPORTED: When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment. The same missing condition, as a state trigger.
card!(
    index = index::CALL_TO_ARMS,
    oracle_id = "72eaaa73-3263-46e9-aa86-5862d3aed20a",
    scryfall_id = "1a772cac-c853-4621-a61f-a9b06f2e2dec",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "white creatures get +1/+1 unconditionally, and it is never sacrificed: no condition reads the most common color among the chosen player's nontoken permanents"
    ),
    faces = &[face!(
        name = "Call to Arms",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseColor, EnterModifier::ChooseOpponent],
    ),],
    abilities = &[static_ability!(WHITE_CREATURE, Modifier::ModifyPT(1, 1))],
);
