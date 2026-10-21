//! `cards/instants/mv_2/rally.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rally() -> CardIndex {
    card_index("274f8860-c8c3-4aeb-ba87-94a5139397d8")
}

/// Rally (274f8860-c8c3-4aeb-ba87-94a5139397d8) — {W}{W} Instant:
/// "Blocking creatures get +1/+1 until end of turn."
#[test]
fn rally_pumps_blocking_creatures_and_leaves_attacking_and_nonblocking_creatures_unchanged() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[plains(), plains(), grizzly_bears(), gray_ogre()])
        .hand(1, &[rally()])
        .start();
    keep_mulligans(&mut engine);

    let attacker = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let bystander = on_battlefield(&engine, P1, gray_ogre()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
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
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    // P0 passes priority in DeclareBlockers; P1 casts Rally.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, rally());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, blocker),
        (3, 3),
        "blocking Grizzly Bears gets +1/+1 from Rally"
    );
    assert_eq!(
        pt(&engine, attacker),
        (2, 2),
        "attacking Grizzly Bears is not a blocking creature"
    );
    assert_eq!(
        pt(&engine, bystander),
        (2, 2),
        "non-blocking Gray Ogre is not a blocking creature"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some()
            && in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "3/3 blocking Grizzly Bears kills 2/2 attacker and survives"
    );
}
