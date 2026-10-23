//! Intervene — {U} — Instant
//! Oracle: Counter target spell that targets a creature.
//! Set: ULG #33 — Urza's Legacy | Scryfall ID: 4b0e3894-5dfe-4d03-9996-eebf96c58168 | Oracle ID: 380b386f-90c8-45c5-a611-1ed6803b52d2
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTERVENE,
    oracle_id = "380b386f-90c8-45c5-a611-1ed6803b52d2",
    scryfall_id = "4b0e3894-5dfe-4d03-9996-eebf96c58168",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Counters target spell; restricting targets to spells that target a creature is not yet in Filter.",
    ),
    faces = &[face!(
        name = "Intervene",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
