//! Silhouette — {1}{U} — Instant
//! Oracle: Choose target creature. If a spell or ability that targets that creature would cause a source to deal damage to that creature this turn, prevent that damage.
//! Set: LEG #77 — Legends | Scryfall ID: e6d6fac6-9a23-465f-a813-92e1ed1cd742 | Oracle ID: 5a074b0a-3a4d-4c6f-b161-5d1281f6136d
// IMPLEMENTED — a temporary shield prevents damage caused by spells or
// abilities targeting the chosen creature, independently of the damage source.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SILHOUETTE,
    oracle_id = "5a074b0a-3a4d-4c6f-b161-5d1281f6136d",
    scryfall_id = "e6d6fac6-9a23-465f-a813-92e1ed1cd742",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Silhouette",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PreventDamageFromTargetingCausesThisTurn {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
