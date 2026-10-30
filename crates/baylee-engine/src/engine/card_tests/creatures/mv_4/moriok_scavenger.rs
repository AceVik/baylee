//! Moriok Scavenger returns only an artifact creature from its owner's graveyard.

use super::*;

fn moriok_scavenger() -> CardIndex {
    card_index("fdccdd9f-6bf9-4339-ad7a-e9dd8f771b22")
}

fn myr_enforcer() -> CardIndex {
    card_index("2d8e1054-654f-42e8-8c29-12c3cf13a3eb")
}

#[test]
fn moriok_scavenger_may_return_an_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 4])
        .hand(0, &[moriok_scavenger(), myr_enforcer(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    for card in [myr_enforcer(), gray_ogre()] {
        let id = in_hand(&e, p0, card).unwrap();
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();
    cast_from_hand(&mut e, p0, moriok_scavenger());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let artifact = in_graveyard(&e, p0, myr_enforcer()).unwrap();
    let ordinary = in_graveyard(&e, p0, gray_ogre()).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&artifact));
    assert!(!options.contains(&ordinary));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![artifact],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, myr_enforcer()).is_some());
}
