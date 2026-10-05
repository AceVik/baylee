//! Sorceress Queen — {1}{B}{B} — Creature — Human Wizard Sorcerer
//! Oracle: {T}: Target creature other than this creature has base power and toughness 0/2 until end of turn.
//! Set: 5ED #194 — Fifth Edition | Scryfall ID: 657979b3-6e4f-41a8-a518-4bd329307770 | Oracle ID: 3e4cb1b2-e2cc-4925-a226-6c6f1501d9c1
// IMPLEMENTED — {T}: target creature other than this one has base power and
// toughness 0/2 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SORCERESS_QUEEN,
    oracle_id = "3e4cb1b2-e2cc-4925-a226-6c6f1501d9c1",
    scryfall_id = "657979b3-6e4f-41a8-a518-4bd329307770",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sorceress Queen",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::SORCERER
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::continuous(
            &Filter::This,
            Modifier::SetPT(0, 2),
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE)),
    )],
);
