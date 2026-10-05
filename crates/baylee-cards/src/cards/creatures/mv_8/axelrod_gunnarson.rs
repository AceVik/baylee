//! Axelrod Gunnarson — {4}{B}{B}{R}{R} — Legendary Creature — Giant
//! Oracle: Trample
//! Oracle: Whenever a creature dealt damage by Axelrod Gunnarson this turn dies, you gain 1 life and Axelrod Gunnarson deals 1 damage to target player or planeswalker.
//! Set: ME3 #143 — Masters Edition III | Scryfall ID: 97f2b387-661e-435f-96c9-d3d5a601caa9 | Oracle ID: c640350b-16a5-4227-87fa-8b24fdb367c7
// PARTIAL — trample and the death trigger's life gain are written; the trigger's damage clause is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AXELROD_GUNNARSON,
    oracle_id = "c640350b-16a5-4227-87fa-8b24fdb367c7",
    scryfall_id = "97f2b387-661e-435f-96c9-d3d5a601caa9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "the death trigger's damage clause targets \"target player or \
         planeswalker\", and no TargetSpec spans the seats with a restricted \
         object set: AnyTarget also offers creatures and battles, and \
         OpponentOrObject offers only opponents"
    ),
    faces = &[face!(
        name = "Axelrod Gunnarson",
        mana_cost = mana!("{4}{B}{B}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(5),
        toughness = Some(5),
    ),],
    // NOT SUPPORTED: "Axelrod Gunnarson deals 1 damage to target player or
    // planeswalker." — the trigger (`Trigger::DiesAfterDamageByThis`) and the
    // life gain are written, but the damage's target set is not:
    // `TargetSpec::AnyTarget` (CR 115.4) is strictly wider — it also offers
    // creatures and battles — and `TargetSpec::OpponentOrObject` is strictly
    // narrower. Dropping the damage rather than shipping a wider offer.
    abilities = &[triggered!(
        Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
        &[Effect::gain_life(1)]
    )],
);
