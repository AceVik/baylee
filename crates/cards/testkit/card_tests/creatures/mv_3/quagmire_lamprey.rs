//! `cards/creatures/mv_3/quagmire_lamprey.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f5b374d8-bf22-44f7-a18a-18c096a84640"
fn quagmire_lamprey() -> CardIndex {
    card_index("f5b374d8-bf22-44f7-a18a-18c096a84640")
}

/// `Quagmire Lamprey` (`Coverage::Implemented`):
/// "Whenever this creature becomes blocked by a creature, put a -1/-1 counter on
/// that creature."
///
/// Verifies that when `Quagmire Lamprey` is blocked by a creature, that blocker
/// gets a -1/-1 counter during the declare-blockers step (killing a 1/1 blocker
/// before combat damage and shrinking a 2/2 blocker so both trade in combat).
#[test]
fn quagmire_lamprey_puts_minus_one_minus_one_counter_on_each_blocking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9508, swamp())
        .battlefield(0, &[quagmire_lamprey()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let lamprey = on_battlefield(&engine, p0, quagmire_lamprey()).expect("P0 Quagmire Lamprey");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Llanowar Elves");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lamprey, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, lamprey)],
            },
        )
        .unwrap();

    // Resolve the BecomesBlockedBy trigger in the declare-blockers step.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "1/1 blocker dies to the -1/-1 counter in the declare-blockers step before combat damage"
    );
    assert!(
        on_battlefield(&engine, p0, quagmire_lamprey()).is_some(),
        "Quagmire Lamprey survives because the 1/1 blocker died before combat damage"
    );
}
