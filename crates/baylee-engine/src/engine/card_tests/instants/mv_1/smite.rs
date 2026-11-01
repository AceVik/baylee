//! `cards/instants/mv_1/smite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn smite() -> CardIndex {
    card_index("367379f2-e1e4-48e7-b2c0-1003de55adbe")
}

/// Smite (`367379f2-e1e4-48e7-b2c0-1003de55adbe`) — {W} Instant (Partial):
/// "Destroy target blocked creature."
#[test]
fn smite_destroys_target_blocked_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[index::HILL_GIANT, plains()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[smite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let blocker = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    let smite_card = in_hand(&engine, P0, smite()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority after attack")
    };
    assert!(
        !legal.castable.contains(&smite_card) && !legal.payable.contains(&smite_card),
        "attacker is not blocked before declarations"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    cast_from_hand(&mut engine, P0, smite());
    aim_at(&mut engine, P0, attacker);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::HILL_GIANT).is_none(),
        "Smite should destroy the blocked Hill Giant"
    );
}
