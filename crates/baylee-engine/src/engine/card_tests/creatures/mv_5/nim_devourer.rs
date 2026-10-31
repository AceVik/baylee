//! Nim Devourer's upkeep-only graveyard return and subsequent sacrifice.

use super::*;

fn nim_devourer() -> CardIndex {
    card_index("89075fbe-cd8a-4054-abab-fcaa2aedddfb")
}

#[test]
fn nim_devourer_returns_during_upkeep_then_sacrifices_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), gray_ogre(), quiet_artifact()])
        .hand(0, &[nim_devourer()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let devourer = in_hand(&e, p0, nim_devourer()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            devourer,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority")
    };
    assert!(
        !legal.abilities.contains(&(devourer, 1)),
        "not at main phase"
    );
    assert_eq!(pt(&e, on_battlefield(&e, p0, gray_ogre()).unwrap()), (2, 2));

    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, nim_devourer(), 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let returned = on_battlefield(&e, p0, nim_devourer()).unwrap();
    assert_eq!(pt(&e, returned), (5, 1), "one controlled artifact counts");
    assert!(in_graveyard(&e, p0, gray_ogre()).is_some());
}
