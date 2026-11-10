//! Panic — {R} — Instant
//! Oracle: Cast this spell only during combat before blockers are declared.
//! Oracle: Target creature can't block this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ME2 #145 — Masters Edition II | Scryfall ID: c3e48eae-7cae-44f0-b009-71f37e698756 | Oracle ID: e98429a7-a83f-4b69-afad-9eb68b09e38e
use baylee_cards_dsl::prelude::*;

card!(
    index = index::PANIC,
    oracle_id = "e98429a7-a83f-4b69-afad-9eb68b09e38e",
    scryfall_id = "c3e48eae-7cae-44f0-b009-71f37e698756",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Panic",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::CantBlock(&Filter::Any),
                Duration::UntilEndOfTurn,
            ),
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::BeforeStep(StepKind::DeclareBlockers),
        ]))
    )],
);
