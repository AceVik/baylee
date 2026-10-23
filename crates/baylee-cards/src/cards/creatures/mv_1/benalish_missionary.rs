//! Benalish Missionary — {W} — Creature — Human Cleric
//! Oracle: {1}{W}, {T}: Prevent all combat damage that would be dealt by target blocked creature this turn.
//! Set: WTH #10 — Weatherlight | Scryfall ID: e9ac1992-6212-4f05-af16-c892dfc40643 | Oracle ID: 84fdcfd3-2b22-4570-af34-7e3f55f97466
// PARTIAL: {1}{W}, {T}: prevents all combat damage that would be dealt by target blocked attacking creature this turn, but only during the declare blockers step; it cannot be activated later in combat (after first-strike damage, say), because "blocked" is read as "not unblocked", which is true of every attacker before blockers are declared.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::Unblocked),
]);

card!(
    index = index::BENALISH_MISSIONARY,
    oracle_id = "84fdcfd3-2b22-4570-af34-7e3f55f97466",
    scryfall_id = "e9ac1992-6212-4f05-af16-c892dfc40643",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{1}{W}, {T}: prevents all combat damage that would be dealt by target blocked attacking creature this turn, but only during the declare blockers step; it cannot be activated later in combat (after first-strike damage, say), because \"blocked\" is read as \"not unblocked\", which is true of every attacker before blockers are declared",
    ),
    faces = &[face!(
        name = "Benalish Missionary",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{W}", TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFromIt,
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&BLOCKED_CREATURE)),
        // NOT SUPPORTED: activating it after the declare blockers step, while the attacker is
        // still a blocked creature. Before blockers are declared `Not(Unblocked)` holds for every
        // attacker, so the step is the only moment "blocked" is read right.
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers)),
    )],
);
