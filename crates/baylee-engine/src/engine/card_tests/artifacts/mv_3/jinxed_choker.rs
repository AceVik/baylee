//! Jinxed Choker's counter choices, damage and control transfer.

use super::*;

fn jinxed_choker() -> CardIndex {
    card_index("3241abc1-d739-4ac3-93d1-7fc2898b943b")
}

#[test]
fn jinxed_choker_adds_and_removes_charge_counters() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                jinxed_choker(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let choker = on_battlefield(&e, p0, jinxed_choker()).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, jinxed_choker(), 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(choker)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
    activate(&mut e, p0, jinxed_choker(), 3);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(choker)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );
}

#[test]
fn jinxed_choker_damages_at_upkeep_and_changes_control_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[jinxed_choker()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let choker = on_battlefield(&e, p0, jinxed_choker()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(choker)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 2);
    e.refresh_offer();
    for (controller, recipient) in [(p0, p1), (p1, p0)] {
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        e.apply(
            controller,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![recipient],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(choker).unwrap().controller, recipient);
        reach_their_main_phase(&mut e, recipient);
    }
    assert_eq!(
        e.state()
            .object(choker)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        4
    );
    assert_eq!(
        e.state().players[0].life,
        16,
        "four counters on the next upkeep"
    );
}
