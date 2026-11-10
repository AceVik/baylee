//! General's Regalia — {3} — Artifact
//! Oracle: {3}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature you control instead.
//! Set: MMQ #295 — Mercadian Masques | Scryfall ID: fb99d982-8ab1-4d6a-ba24-58ac23a9b9e7 | Oracle ID: cceba6f3-b1c0-45ee-826c-b871d24c7208
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GENERAL_S_REGALIA,
    oracle_id = "cceba6f3-b1c0-45ee-826c-b871d24c7208",
    scryfall_id = "fb99d982-8ab1-4d6a-ba24-58ac23a9b9e7",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "General's Regalia",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[Effect::RedirectNextChosenSourceDamageToTarget {
            target: TargetSpec::Object(&Filter::YOUR_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )],
);
