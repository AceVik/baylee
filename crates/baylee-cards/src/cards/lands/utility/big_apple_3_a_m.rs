//! Big Apple, 3 a.m. — (no cost) — Land
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {5}, {T}: Create a 1/1 black Rat creature token for each opponent you have.
//! Set: TMC #42 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: b9cc16f9-fea3-4527-9b81-3c84ce9c5e01 | Oracle ID: dd01ef1f-f6be-498f-82e0-dc04833e685f
// PARTIAL — enters tapped and chooses a color (EnterModifier::Tapped +
// ChooseColor), which it taps for; the Rat token clause is dropped below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BIG_APPLE_3_A_M,
    oracle_id = "dd01ef1f-f6be-498f-82e0-dc04833e685f",
    scryfall_id = "b9cc16f9-fea3-4527-9b81-3c84ce9c5e01",
    faces = &[face!(
        name = "Big Apple, 3 a.m.",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped, EnterModifier::ChooseColor],
    )],
    coverage = Coverage::Partial(
        "{5}, {T}: Create a 1/1 black Rat creature token for each opponent you have — \
         no Amount counts seats, and no Filter can match a player",
    ),
    // NOT SUPPORTED: {5}, {T}: Create a 1/1 black Rat creature token for each
    // opponent you have — "for each opponent" needs an amount that counts
    // players, and every `Amount` counts objects a `Filter` matches; a player
    // has no characteristics for a filter to read, so the ability comes off
    // the card rather than resolving for a fixed number.
    abilities = &[mana_ability!(&[Effect::mana_chosen()])],
);
