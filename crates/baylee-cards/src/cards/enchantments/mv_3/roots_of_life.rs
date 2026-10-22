//! Roots of Life — {1}{G}{G} — Enchantment
//! Oracle: As this enchantment enters, choose Island or Swamp.
//! Oracle: Whenever a land of the chosen type an opponent controls becomes tapped, you gain 1 life.
//! Set: MIR #237 — Mirage | Scryfall ID: 26724a51-87dd-4159-b012-71598e4cf5eb | Oracle ID: 29ec9b49-fbe5-49ac-8ad6-7159cb88bc66

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_CHOSEN_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::ControlledByOpponent,
    Filter::MatchesChosenTypeOfSource,
]);

card!(
    index = index::ROOTS_OF_LIFE,
    oracle_id = "29ec9b49-fbe5-49ac-8ad6-7159cb88bc66",
    scryfall_id = "26724a51-87dd-4159-b012-71598e4cf5eb",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Roots of Life",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseSubtypeFrom(&[
            subtypes::land::ISLAND,
            subtypes::land::SWAMP,
        ])],
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTapped(&OPPONENT_CHOSEN_LAND),
        &[Effect::gain_life(1)],
    )],
);
