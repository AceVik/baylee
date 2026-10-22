//! `cards/creatures/artifacts/mv_3/horrible_hordes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HORRIBLE_HORDES};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Horrible Hordes (cf04f2cd-d9f8-4e0c-adaf-6d38bc14dd7a) — {3} 2/2:
/// "Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end
/// of turn for each creature blocking it beyond the first.)"
#[test]
fn horrible_hordes_triggers_rampage_when_blocked_by_multiple_creatures() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[HORRIBLE_HORDES])
        .battlefield(1, &[GRIZZLY_BEARS, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let hordes = on_battlefield(&engine, P0, HORRIBLE_HORDES).unwrap();
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
