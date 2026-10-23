//! `cards/creatures/artifacts/mv_3/horrible_hordes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, WALL_OF_WOOD};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn horrible_hordes() -> CardIndex {
    card_index("79bfef02-3586-424a-aff9-a6ec0d4f1135")
}

/// Horrible Hordes (79bfef02-3586-424a-aff9-a6ec0d4f1135) — {3} 2/2:
/// "Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end
/// of turn for each creature blocking it beyond the first.)"
#[test]
fn horrible_hordes_triggers_rampage_when_blocked_by_multiple_creatures() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[horrible_hordes()])
        .battlefield(1, &[GRIZZLY_BEARS, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let hordes = on_battlefield(&engine, P0, horrible_hordes()).unwrap();
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
                attackers: vec![(hordes, Defender::Player(P1))],
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
                blockers: vec![(blockers[0], hordes), (blockers[1], hordes)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(hordes).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));
}

/// "for each creature blocking it beyond the first": one blocker (a Wall of
/// Wood) gives no bonus; the Hordes stay 2/2.
#[test]
fn horrible_hordes_gets_no_bonus_from_a_single_blocker() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[horrible_hordes()])
        .battlefield(1, &[WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);

    let hordes = on_battlefield(&engine, P0, horrible_hordes()).unwrap();
    let wall = on_battlefield(&engine, P1, WALL_OF_WOOD).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(hordes, Defender::Player(P1))],
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
                blockers: vec![(wall, hordes)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(pt(&engine, hordes), (2, 2));
}
