//! Juxtapose — {3}{U} — Sorcery
//! Oracle: You and target player exchange control of the creature you each control with the greatest mana value. Then exchange control of artifacts the same way. If two or more permanents a player controls are tied for greatest, their controller chooses one of them.
//! Set: ME1 #41 — Masters Edition | Scryfall ID: 3b894a51-6d4e-4e0f-b42a-dbab594fb81d | Oracle ID: a52ccb40-e950-4cd0-93ab-50daa52b03f3
// PARTIAL — the exchange is off the card: nothing selects the greatest-mana-
// value permanent, and the printed tie-break is that player's choice.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUXTAPOSE,
    oracle_id = "a52ccb40-e950-4cd0-93ab-50daa52b03f3",
    scryfall_id = "3b894a51-6d4e-4e0f-b42a-dbab594fb81d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Juxtapose",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no effect chooses the creature or artifact with the greatest mana \
         value, and the printed tie-break hands that choice to the permanents' \
         controller",
    ),
    // NOT SUPPORTED: "You and target player exchange control of the creature
    // you each control with the greatest mana value. Then exchange control of
    // artifacts the same way." — `Effect::ExchangeControl` and
    // `ExchangeControlOrSacrifice` swap permanents named as targets; nothing
    // picks a player's permanent with the greatest mana value, and no amount
    // or filter reads a "greatest" among permanents.
    // NOT SUPPORTED: "If two or more permanents a player controls are tied for
    // greatest, their controller chooses one of them." — the choice belongs to
    // the permanents' controller, and no effect asks a player to pick among
    // their own permanents for an exchange.
    abilities = &[],
);
