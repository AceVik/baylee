//! Cuombajj Witches — {B}{B} — Creature — Human Wizard
//! Oracle: {T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice.
//! Set: CMR #116 — Commander Legends | Scryfall ID: 6a26e910-275a-4981-831b-bfed936a7e3f | Oracle ID: 638eeb16-9e0e-4cc6-b97e-8ff0df81ca58
// PARTIAL — the first damage is written; the opponent's-choice damage is off
// the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CUOMBAJJ_WITCHES,
    oracle_id = "638eeb16-9e0e-4cc6-b97e-8ff0df81ca58",
    scryfall_id = "6a26e910-275a-4981-831b-bfed936a7e3f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cuombajj Witches",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "the second damage — 1 damage to any target of an opponent's choice — \
         is not written: no TargetSpec has a chooser, so the printed \
         opponent's-choice target cannot be named"
    ),
    // NOT SUPPORTED: "…and 1 damage to any target of an opponent's choice."
    // — `TargetSpec::AnyTarget` on a `second_targets` list is the nearest
    // piece, but a `TargetSpec` says what may be chosen and never who
    // chooses: every target of an ability is chosen by its controller
    // (CR 601.2c), so that spelling would let the Witches' controller pick
    // both targets. Nothing in the vocabulary hands a target question to an
    // opponent.
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget
        }],
        target = Some(TargetSpec::AnyTarget)
    )],
);
