//! Disharmony — {2}{R} — Instant
//! Oracle: Cast this spell only during combat before blockers are declared.
//! Oracle: Untap target attacking creature and remove it from combat. Gain control of that creature until end of turn.
//! Set: ME3 #95 — Masters Edition III | Scryfall ID: 37fd8d07-0eed-4b41-9468-b524439cd204 | Oracle ID: fc4f1aa1-e252-4454-9ad9-d41682dff13b
// PARTIAL — the cast window, the untap and the control change are all
// implemented; "remove it from combat" is dropped because no effect or
// modifier removes a permanent from combat.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISHARMONY,
    oracle_id = "fc4f1aa1-e252-4454-9ad9-d41682dff13b",
    scryfall_id = "37fd8d07-0eed-4b41-9468-b524439cd204",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "\"remove it from combat\" is not in the vocabulary: no effect or modifier removes \
         a permanent from combat"
    ),
    faces = &[face!(
        name = "Disharmony",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "and remove it from combat" — no effect or modifier
    // removes a permanent from combat.
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::GainControl,
                Duration::UntilEndOfTurn
            ),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE
        ))),
        condition = Some(Condition::All(&[
            Condition::DuringCombat,
            Condition::BeforeStep(StepKind::DeclareBlockers),
        ]))
    )],
);
