//! Grove of the Guardian — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{G}{W}, {T}, Tap two untapped creatures you control, Sacrifice this land: Create an 8/8 green and white Elemental creature token with vigilance.
//! Set: RTR #240 — Return to Ravnica | Scryfall ID: 3cf60ca0-e01f-499c-8d04-d59050f38c33 | Oracle ID: f746612a-fbed-44ca-b2cc-5928e10cf4bb
// PARTIAL — the mana ability is built; the Elemental ability is off the
// card (see the NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GROVE_OF_THE_GUARDIAN,
    oracle_id = "f746612a-fbed-44ca-b2cc-5928e10cf4bb",
    scryfall_id = "3cf60ca0-e01f-499c-8d04-d59050f38c33",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Grove of the Guardian", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "activation cost \"Tap two untapped creatures you control\": \
         CostPart::TapOther names exactly one permanent and carries no count"
    ),
    abilities = &[
        // NOT SUPPORTED: "{3}{G}{W}, {T}, Tap two untapped creatures you
        // control, Sacrifice this land: Create an 8/8 green and white
        // Elemental creature token with vigilance." — the cost taps TWO
        // creatures, and `cost!(…, TapOther(filter))` taps one. Written with
        // a single `TapOther` the ability would be strictly cheaper and would
        // trade two creatures for one, so the ability comes off the card
        // rather than being approximated into a different one.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
