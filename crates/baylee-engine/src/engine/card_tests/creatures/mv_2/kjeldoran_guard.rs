//! `cards/creatures/mv_2/kjeldoran_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kjeldoran_guard() -> CardIndex {
    card_index("bd699cf8-56c7-4375-920e-8dd22c83a82c")
}

/// Partial Kjeldoran Guard keeps its 1/1 body. Its optional pump is omitted
/// together with the linked sacrifice until defending-player selection can
/// happen during activation, including before attackers are declared.
#[test]
fn kjeldoran_guard_attacks_for_one_without_the_unsupported_activation() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[kjeldoran_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, kjeldoran_guard());
    pass_until(&mut engine, stack_is_empty);
    let guard = on_battlefield(&engine, p0, kjeldoran_guard()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatBegin
    });
    assert!(
        !ability_offered(&engine, guard, 0),
        "the omitted ability is unavailable even after summoning sickness ends"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: guard,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the unsupported activation is refused"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guard, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the body deals 1 damage"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert!(on_battlefield(&engine, p0, kjeldoran_guard()).is_some());
}
