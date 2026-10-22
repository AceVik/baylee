//! `cards/creatures/mv_5/jungle_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, JUNGLE_WURM};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Jungle Wurm (f6862a0d-c928-4d62-9146-e10173044f41) — {3}{G}{G} 5/5:
/// "Whenever this creature becomes blocked, it gets -1/-1 until end of turn
/// for each creature blocking it beyond the first."
#[test]
fn jungle_wurm_shrinks_by_one_for_each_blocker_beyond_the_first() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[JUNGLE_WURM])
        .battlefield(1, &[GRIZZLY_BEARS, GRIZZLY_BEARS, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let wurm = on_battlefield(&engine, P0, JUNGLE_WURM).unwrap();
    let blockers: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == P1 && o.card.is_some_and(|c| c.index == GRIZZLY_BEARS)
        })
        .collect();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wurm, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![
                    (blockers[0], wurm),
                    (blockers[1], wurm),
                    (blockers[2], wurm),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(wurm).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));
}
