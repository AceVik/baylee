//! Oracle scenarios for Abrade.
#[allow(clippy::wildcard_imports)]
use super::*;

fn abrade() -> CardIndex {
    card_index("f9db72dc-9a5b-48a4-a86e-7464d9a2166a")
}

// Checklist: each mode resolves independently; creature mode deals exactly 3
// damage to a 4/4 and leaves it alive; artifact mode destroys an artifact.
#[test]
fn abrade_resolves_each_mode_against_its_named_category() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[abrade(), abrade()])
        .battlefield(1, &[fangren_hunter(), quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let creature = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    let artifact = on_battlefield(&e, p1, quiet_artifact()).unwrap();
    let elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut e, p0, abrade());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("Abrade offers its two modes")
    };
    assert_eq!(
        options
            .iter()
            .filter(|o| matches!(o.kind, CastModeKind::Mode(_)))
            .count(),
        2
    );
    let mode = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(0))
        .unwrap();
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("creature target")
    };
    assert!(options.contains(&creature));
    assert!(
        options.contains(&elf),
        "any creature is legal, including a 1/1"
    );
    assert!(!options.contains(&artifact));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![creature],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(creature).unwrap().damage,
        3,
        "Abrade deals exactly three damage"
    );
    assert!(
        on_battlefield(&e, p1, fangren_hunter()).is_some(),
        "the 4/4 survives three damage"
    );
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());

    cast_from_hand(&mut e, p0, abrade());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("second mode choice")
    };
    let mode = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(1))
        .unwrap();
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("artifact target")
    };
    assert_eq!(options, vec![artifact]);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![artifact],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, quiet_artifact()).is_some());
    assert!(on_battlefield(&e, p1, llanowar_elves()).is_some());
}
