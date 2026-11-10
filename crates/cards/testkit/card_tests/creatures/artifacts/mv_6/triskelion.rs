//! `cards/creatures/artifacts/mv_6/triskelion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Triskelion — {6} — artifact creature, 1/1: "This creature enters with
/// three +1/+1 counters on it. Remove a +1/+1 counter from this creature:
/// It deals 1 damage to any target."
///
/// "Enters with … counters" is a replacement effect that puts the counters
/// on as it arrives (CR 614.1c), so the printed 1/1 sits on the battlefield
/// as a 4/4 and no state-based action ever sees it without them. Each
/// activation removes its counter as the cost, paid after the target is
/// named (CR 601.2h), and once the third is gone the cost of a fourth
/// cannot be paid at all (CR 118.3) — which is the whole rule behind "four
/// activations are impossible", not a special limit printed on the card.
#[test]
fn triskelion_enters_with_three_counters_and_spends_only_those_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[triskelion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Forests pay {6}; the entry replacement is the only thing that
    // lets the printed 1/1 survive its own arrival.
    cast_from_hand(&mut engine, p0, triskelion());
    pass_until(&mut engine, stack_is_empty);

    let trike = on_battlefield(&engine, p0, triskelion()).expect("it resolved");
    assert_eq!(
        counters_on(&engine, trike, CounterKind::P1P1),
        3,
        "three +1/+1 counters arrive with it (CR 614.1c)"
    );
    assert_eq!(
        pt(&engine, trike),
        (4, 4),
        "printed 1/1 plus three counters"
    );

    // First activation: the target is chosen before the cost is paid
    // (CR 601.2h), so the counter is still on the body while the question
    // is open, and "any target" includes a player (CR 115.4).
    activate(&mut engine, p0, triskelion(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("the ping targets, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "any target includes a player (CR 115.4)"
    );
    assert_eq!(
        counters_on(&engine, trike, CounterKind::P1P1),
        3,
        "the target is named before the counter is paid (CR 601.2h)"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is a legal target");
    assert_eq!(
        counters_on(&engine, trike, CounterKind::P1P1),
        2,
        "removing the counter is the whole cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19, "one damage");

    for (remaining, life) in [(1_u16, 18), (0, 17)] {
        activate(&mut engine, p0, triskelion(), 0);
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .expect("the opponent is a legal target");
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            counters_on(&engine, trike, CounterKind::P1P1),
            remaining,
            "one counter per activation"
        );
        assert_eq!(engine.state().players[1].life, life);
    }

    assert_eq!(pt(&engine, trike), (1, 1), "back to the printed 1/1");
    assert!(
        !priority_offer(&engine).abilities.contains(&(trike, 0)),
        "with no counter to remove the cost cannot be paid (CR 118.3)"
    );
    assert!(
        on_battlefield(&engine, p0, triskelion()).is_some(),
        "and the 1/1 is still alive"
    );
}
