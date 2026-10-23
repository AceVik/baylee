//! Cannibalize — {1}{B} — Sorcery
//! Oracle: Choose two target creatures controlled by the same player. Exile one of those creatures and put two +1/+1 counters on the other.
//! Set: TPR #83 — Tempest Remastered | Scryfall ID: 9cdf1311-1660-4ad0-a383-f3ff508ff574 | Oracle ID: bf044267-92b9-4983-964f-2bc4ecf76bce
// PARTIAL: exiles target creature; choosing two target creatures controlled by the same player and putting two +1/+1 counters on the other is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CANNIBALIZE,
    oracle_id = "bf044267-92b9-4983-964f-2bc4ecf76bce",
    scryfall_id = "9cdf1311-1660-4ad0-a383-f3ff508ff574",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "exiles target creature; choosing two target creatures controlled by the same player and putting two +1/+1 counters on the other is not supported"
    ),
    faces = &[face!(
        name = "Cannibalize",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
