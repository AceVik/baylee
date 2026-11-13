//! Dream's Grip — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Tap target permanent.
//! Oracle: • Untap target permanent.
//! Oracle: Entwine {1} (Choose both if you pay the entwine cost.)
//! Set: MRD #34 — Mirrodin | Scryfall ID: 7ffaa6a2-7c86-45b4-8892-b837e05f11a6 | Oracle ID: 8b239262-c392-4ec6-9ae1-8b4a076e4bf0
use baylee_cards_dsl::prelude::*;

card!(
    index = index::DREAM_S_GRIP,
    oracle_id = "8b239262-c392-4ec6-9ae1-8b4a076e4bf0",
    scryfall_id = "7ffaa6a2-7c86-45b4-8892-b837e05f11a6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dream's Grip",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
        entwine = Some(cost!("{1}")),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::TapTarget],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
            ),
            mode!(
                &[Effect::UntapTarget],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
            ),
        ],
    }],
);
