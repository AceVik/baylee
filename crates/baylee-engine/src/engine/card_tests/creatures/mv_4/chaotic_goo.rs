//! `cards/creatures/mv_4/chaotic_goo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chaotic_goo() -> CardIndex {
    card_index("d1005625-0a78-4f8e-bf51-80458fc0e9d6")
}

/// Chaotic Goo (d1005625-0a78-4f8e-bf51-80458fc0e9d6) — {2}{R}{R} Creature — Ooze:
/// "This creature enters with three +1/+1 counters on it. At the beginning of your upkeep, you may flip a coin. If you win the flip, put a +1/+1 counter on this creature. If you lose the flip, remove a +1/+1 counter from this creature."
#[test]
fn chaotic_goo_enters_with_three_counters_and_flips_at_upkeep() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[chaotic_goo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, chaotic_goo());
    pass_until(&mut engine, stack_is_empty);

    let goo = on_battlefield(&engine, P0, chaotic_goo()).unwrap();
    assert_eq!(pt(&engine, goo), (3, 3));

    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let (p, t) = pt(&engine, goo);
    assert!(
        (p == 4 && t == 4) || (p == 2 && t == 2),
        "expected (4, 4) or (2, 2) after coin flip, got ({p}, {t})"
    );
}
