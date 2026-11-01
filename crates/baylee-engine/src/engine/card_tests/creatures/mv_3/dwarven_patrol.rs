//! Oracle scenarios for Dwarven Patrol.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn dwarven_patrol() -> CardIndex {
    card_index("4369adc8-d8e0-4d51-b2a9-7e4a8dfd9f8f")
}

#[test]
fn dwarven_patrol_does_not_untap_in_untap_step_and_untaps_when_you_cast_a_nonred_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN, forest(), dwarven_patrol()])
        .hand(0, &[index::SHOCK, giant_growth()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let patrol = on_battlefield(&e, p0, dwarven_patrol()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(patrol)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    // Pass to p1's turn and back to p0's main phase: Dwarven Patrol does not untap during your untap step.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert!(
        e.state()
            .object(patrol)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Dwarven Patrol stays tapped through its controller's untap step"
    );

    // Casting a red spell (Shock targeting p1) does not trigger Dwarven Patrol.
    cast_from_hand(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(patrol)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "casting a red spell does not untap Dwarven Patrol"
    );

    // Casting a nonred spell (Giant Growth targeting Dwarven Patrol) triggers Dwarven Patrol and untaps it.
    cast_with_floating(&mut e, p0, giant_growth());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![patrol],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        !e.state()
            .object(patrol)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "casting a nonred spell untaps Dwarven Patrol"
    );
}
