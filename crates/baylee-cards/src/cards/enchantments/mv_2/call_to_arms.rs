//! Call to Arms — {1}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a color and an opponent.
//! Oracle: White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common.
//! Oracle: When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment.
//! Set: ME3 #4 — Masters Edition III | Scryfall ID: 1a772cac-c853-4621-a61f-a9b06f2e2dec | Oracle ID: 72eaaa73-3263-46e9-aa86-5862d3aed20a
// PARTIAL: most-common-color-among-nontoken-permanents condition and state trigger not supported.

use baylee_cards_dsl::prelude::*;

const WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

// NOT SUPPORTED: As this enchantment enters, choose a color and an opponent.
// NOT SUPPORTED: When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment.
card!(
    index = index::CALL_TO_ARMS,
    oracle_id = "72eaaa73-3263-46e9-aa86-5862d3aed20a",
    scryfall_id = "1a772cac-c853-4621-a61f-a9b06f2e2dec",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "most-common-color-among-nontoken-permanents condition and state trigger not supported"
    ),
    faces = &[face!(
        name = "Call to Arms",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseColor],
    ),],
    abilities = &[static_ability!(WHITE_CREATURE, Modifier::ModifyPT(1, 1))],
);
