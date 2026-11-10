//! Looming Hoverguard puts its artifact target on top of the owner's library.

use super::*;

fn looming_hoverguard() -> CardIndex {
    card_index("e1c49bdb-cfd1-4ebf-90f7-73bd5009f75e")
}

#[test]
fn looming_hoverguard_puts_target_artifact_on_top() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 6])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[looming_hoverguard()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let artifact = on_battlefield(&e, p1, quiet_artifact()).unwrap();
    cast_from_hand(&mut e, p0, looming_hoverguard());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![artifact],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_none());
    let top = e
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    assert_eq!(
        e.state().object(*top).unwrap().card.unwrap().index,
        quiet_artifact()
    );
}
