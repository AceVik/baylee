//! Priest of Yawgmoth — {1}{B} — Creature — Phyrexian Human Cleric
//! Oracle: {T}, Sacrifice an artifact: Add an amount of {B} equal to the sacrificed artifact's mana value.
//! Set: ATQ #19 — Antiquities | Scryfall ID: c9fd4054-42fc-4f95-a6f7-369a5da43dd5 | Oracle ID: cb6465f9-dcf8-4258-aa18-661ad252b58b
// PARTIAL — the mana ability is off the card: its amount can't be read on a mana ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRIEST_OF_YAWGMOTH,
    oracle_id = "cb6465f9-dcf8-4258-aa18-661ad252b58b",
    scryfall_id = "c9fd4054-42fc-4f95-a6f7-369a5da43dd5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Priest of Yawgmoth",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC
        ],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "the payment records the sacrificed artifact's mana value only on a \
         spell or a stack-using activation; a mana ability resolves with \
         `on_stack` set to its source permanent, which carries no \
         `PaidRecord`, so `Amount::SacrificedManaValue` reads 0"
    ),
    // NOT SUPPORTED: "{T}, Sacrifice an artifact: Add an amount of {B} equal
    // to the sacrificed artifact's mana value." — the cost and the amount are
    // both sayable (`cost!(TapSelf, Sacrifice(&Filter::YOUR_ARTIFACT))` and
    // `Effect::mana_dynamic(ManaColor::Black, Amount::SacrificedManaValue)`),
    // but the card's ability is a mana ability (CR 605.1a: no target, it
    // could add mana), and `start_activation` gives a mana ability's
    // `Resolution` `on_stack` = the source permanent (abilities.rs:2279).
    // The payment writes `PaidRecord::sacrificed_mana_value` onto a spell or
    // a stack-using ability object, never onto the permanent, so the amount
    // reads 0. Written as a stack-using `activated!` it would add the right
    // mana but would stop being a mana ability — respondable, and unusable
    // while paying costs — which is not the printed card either.
    abilities = &[],
);
