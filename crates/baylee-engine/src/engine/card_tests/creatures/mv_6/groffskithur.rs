//! Groffskithur returns another copy when it becomes blocked.

use super::*;

fn groffskithur() -> CardIndex {
    card_index("32a8ca5f-0170-4f5d-97cd-63e2b74fd653")
}

#[test]
fn groffskithur_returns_its_namesake_from_graveyard_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[groffskithur()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[groffskithur()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let copy = in_hand(&e, p0, groffskithur()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            copy,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let attacker = on_battlefield(&e, p0, groffskithur()).unwrap();
    let blocker = on_battlefield(&e, p1, gray_ogre()).unwrap();
    attack_and_collect_blocks(&mut e, attacker, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![copy],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, groffskithur()).is_some());
}
