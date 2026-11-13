//! Nesting Grounds — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery.
//! Set: ECC #155 — Lorwyn Eclipsed Commander | Scryfall ID: c25ad164-4076-496d-833c-d607391b1ac8 | Oracle ID: d27bb97d-286b-4947-8d7b-443e4df93319

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NESTING_GROUNDS,
    oracle_id = "d27bb97d-286b-4947-8d7b-443e4df93319",
    scryfall_id = "c25ad164-4076-496d-833c-d607391b1ac8",
    faces = &[face!(name = "Nesting Grounds", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::MoveCounter {
                from: TargetSlot::First,
                to: TargetSlot::Second,
                kind: None
            }],
            target = Some(TargetSpec::Object(&Filter::ControlledByYou)),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);
