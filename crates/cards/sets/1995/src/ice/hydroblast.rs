//! Hydroblast — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Counter target spell if it's red.
//! Oracle: • Destroy target permanent if it's red.
//! Set: EMA #55 — Eternal Masters | Scryfall ID: 4c9c9b16-5567-4473-95e6-622292f77336 | Oracle ID: b94c0336-2ca8-4bb1-bb1f-e9babd1826ad
use baylee_cards_dsl::prelude::*;

static RED: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

card!(
    index = index::HYDROBLAST,
    oracle_id = "b94c0336-2ca8-4bb1-bb1f-e9babd1826ad",
    scryfall_id = "4c9c9b16-5567-4473-95e6-622292f77336",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hydroblast",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::IfTargetMatches {
                    filter: &RED,
                    then: &[Effect::CounterTargetSpell],
                }],
                targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
            ),
            mode!(
                &[Effect::IfTargetMatches {
                    filter: &RED,
                    then: &[Effect::destroy(TargetSpec::Object(&Filter::Any))],
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
            ),
        ],
    }],
);
