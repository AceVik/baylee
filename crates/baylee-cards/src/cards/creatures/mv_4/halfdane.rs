//! Halfdane — {1}{W}{U}{B} — Legendary Creature — Shapeshifter
//! Oracle: At the beginning of your upkeep, Halfdane's base power and toughness become equal to the power and toughness of target creature other than Halfdane until the end of your next upkeep.
//! Set: ME3 #150 — Masters Edition III | Scryfall ID: cac1e57a-0ef5-442b-a492-059fc6ff3dfc | Oracle ID: 2ee13155-a3b8-4cad-8e68-5d2c5aba3bb4
// PARTIAL — the upkeep ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HALFDANE,
    oracle_id = "2ee13155-a3b8-4cad-8e68-5d2c5aba3bb4",
    scryfall_id = "cac1e57a-0ef5-442b-a492-059fc6ff3dfc",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "no Amount reads the target's toughness and no Duration ends at the \
         controller's next upkeep"
    ),
    faces = &[face!(
        name = "Halfdane",
        mana_cost = mana!("{1}{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(3),
        toughness = Some(3),
    ),],
    // NOT SUPPORTED: "At the beginning of your upkeep, Halfdane's base power
    // and toughness become equal to the power and toughness of target
    // creature other than Halfdane until the end of your next upkeep." — the
    // upkeep trigger, the "other than this creature" target and
    // `Effect::SetPTFilter` all exist, but `SetPTFilter` sets both halves at
    // once while no `Amount` reads the target's toughness (`Amount::TargetPower`
    // reads power alone), and "until the end of your next upkeep" has no
    // `Duration` (the nearest, `UntilYourNextTurn` and `UntilYourNextUntapStep`,
    // end at other moments of the turn).
    abilities = &[],
);
