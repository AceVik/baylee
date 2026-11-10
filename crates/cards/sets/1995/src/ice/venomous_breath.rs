//! Venomous Breath — {3}{G} — Instant
//! Oracle: Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn.
//! Set: MMQ #281 — Mercadian Masques | Scryfall ID: 4797556c-df74-4e13-b8fb-8b0b58c92b4c | Oracle ID: fc73bd1b-c713-4825-9f2d-f6ae95dddf09

use baylee_cards_dsl::prelude::*;

static BLOCKED_OR_WAS_BLOCKED_BY_IT: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[
        Filter::BlockedReferenceThisTurn,
        Filter::BlockedByReferenceThisTurn,
    ]),
]);

card!(
    index = index::VENOMOUS_BREATH,
    oracle_id = "fc73bd1b-c713-4825-9f2d-f6ae95dddf09",
    scryfall_id = "4797556c-df74-4e13-b8fb-8b0b58c92b4c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Venomous Breath",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::AtEndOfCombatThisTurn {
            about: TargetSpec::Object(&Filter::CREATURE),
            effects: &[Effect::destroy_all(&BLOCKED_OR_WAS_BLOCKED_BY_IT)],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
