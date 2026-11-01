//! `cards/instants/mv_2/provoke.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn provoke() -> CardIndex {
    card_index("731821b1-89fd-4166-94b8-35a1364e4d9d")
}

/// Provoke (`731821b1-89fd-4166-94b8-35a1364e4d9d`) — {1}{G} Instant (Implemented):
/// "Untap target creature you don't control. That creature blocks this turn if able. Draw a card."
#[test]
fn provoke_untaps_opponent_creature_forces_block_and_draws_a_card() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::HILL_GIANT, forest(), forest()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[provoke()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let blocker = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(blocker)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast_from_hand(&mut engine, P0, provoke());
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![attacker],
                    players: vec![]
                }
            )
            .is_err()
    );
    aim_at(&mut engine, P0, blocker);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(blocker)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Provoke should untap the targeted creature"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "Provoke should draw a card to replace itself"
    );

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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "provoked creature must block if able"
    );
    lk::block(&mut engine, &[(blocker, attacker)]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 20);
}
