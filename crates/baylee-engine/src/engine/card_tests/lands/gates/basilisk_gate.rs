//! `cards/lands/gates/basilisk_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Basilisk Gate's pump counts **Gates you control**, and the number is read
/// as the ability resolves. Two on the battlefield is what makes that
/// readable at all: at one Gate, "the number of Gates you control" and "one"
/// and "the source itself" are the same number, and a card counting any of
/// the three would pass. The source counts itself, so two Gates is +2/+2.
///
/// The `{2}` is paid from lands that are not the Gate, because its own tap
/// symbol is part of the cost — a test that let `tap_all_mana` take the Gate
/// would be asserting that an ability nobody could activate does nothing.
#[test]
fn basilisk_gate_pumps_by_the_number_of_gates_and_counts_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                basilisk_gate(),
                basilisk_gate(),
                quiet_creature(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("the creature to point at");
    let base = pt(&engine, creature);
    let gate = on_battlefield(&engine, p0, basilisk_gate()).expect("a Gate to activate");

    tap_mana_except(&mut engine, p0, gate);
    activate(&mut engine, p0, basilisk_gate(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the pump asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&creature),
        "\"target creature\" — the creature on the board is one"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![creature],
                players: vec![],
            },
        )
        .expect("a target the ability offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        pt(&engine, creature),
        (base.0 + 2, base.1 + 2),
        "two Gates you control, and the Gate that paid counts itself"
    );
    assert!(
        is_tapped(&engine, gate),
        "its own tap symbol was part of the price"
    );
}
