//! `cards/creatures/mv_4/stalking_tiger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn stalking_tiger() -> CardIndex {
    card_index("e6af1429-276f-41a8-88d0-062661dc0cd4")
}

/// Stalking Tiger (`{3}{G}`, `Coverage::Partial`): the body is what is played,
/// cast for `{3}{G}` and attacking for 3 once it is out of summoning sickness.
/// "Can't be blocked by more than one creature" is not played.
#[test]
fn stalking_tiger_is_cast_and_attacks_for_3() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[stalking_tiger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, stalking_tiger());
    pass_until(&mut engine, stack_is_empty);
    let tiger = on_battlefield(&engine, p0, stalking_tiger()).unwrap();
    assert_eq!(pt(&engine, tiger), (3, 3));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tiger, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(engine.state().players[1].life, 17);
}
