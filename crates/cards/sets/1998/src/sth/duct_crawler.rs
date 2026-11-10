//! Duct Crawler — {R} — Creature — Insect
//! Oracle: {1}{R}: Target creature can't block this creature this turn.
//! Set: 10E #198 — Tenth Edition | Scryfall ID: d913e4d2-3502-41f3-871e-59b3b13df58a | Oracle ID: d47a7aa5-3b64-4ff8-bb10-e46a34d1993c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DUCT_CRAWLER,
    oracle_id = "d47a7aa5-3b64-4ff8-bb10-e46a34d1993c",
    scryfall_id = "d913e4d2-3502-41f3-871e-59b3b13df58a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Duct Crawler",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}"),
        &[Effect::ModifyTargets {
            modifier: Modifier::CantBlock(&Filter::This),
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
