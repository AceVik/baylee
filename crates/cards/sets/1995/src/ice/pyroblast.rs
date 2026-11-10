//! Pyroblast — {R} — Instant
//! Oracle: Choose one —
//! Oracle: • Counter target spell if it's blue.
//! Oracle: • Destroy target permanent if it's blue.
//! Set: EMA #142 — Eternal Masters | Scryfall ID: b029eb9a-dd7a-40c2-96c4-0063d9cc002c | Oracle ID: ecc435e2-deb1-420a-a79f-01dd08747314
use baylee_cards_dsl::prelude::*;

static BLUE: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

card!(
    index = index::PYROBLAST,
    oracle_id = "ecc435e2-deb1-420a-a79f-01dd08747314",
    scryfall_id = "b029eb9a-dd7a-40c2-96c4-0063d9cc002c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pyroblast",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::IfTargetMatches {
                    filter: &BLUE,
                    then: &[Effect::CounterTargetSpell],
                }],
                targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
            ),
            mode!(
                &[Effect::IfTargetMatches {
                    filter: &BLUE,
                    then: &[Effect::destroy(TargetSpec::Object(&Filter::Any))],
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
            ),
        ],
    }],
);
