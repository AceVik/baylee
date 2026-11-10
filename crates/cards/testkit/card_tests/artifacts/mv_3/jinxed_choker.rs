//! Jinxed Choker's counter choices, damage and control transfer.

use super::*;

fn jinxed_choker() -> CardIndex {
    card_index("3241abc1-d739-4ac3-93d1-7fc2898b943b")
}

fn charge(e: &Engine<RegistryLookup>, choker: ObjectId) -> u16 {
    e.state()
        .object(choker)
        .unwrap()
        .counters
        .get(CounterKind::Charge)
}

/// Activates the one `{3}` ability and answers its resolution-time choice:
/// mode 0 puts a counter on, mode 1 takes one off (2004-12-01 ruling).
fn activate_and_choose(e: &mut Engine<RegistryLookup>, mode: usize) {
    let p0 = PlayerId::new(0);
    activate(e, p0, jinxed_choker(), 2);
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseCastMode { .. }));
    assert!(
        matches!(e.pending(), Pending::ChooseCastMode { player, .. } if *player == p0),
        "the controller chooses add or remove on resolution"
    );
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(e, stack_is_empty);
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
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority")
    };
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(source, _)| *source == choker)
            .count(),
        1,
        "put or remove is one {{3}} ability, not two"
    );
    activate_and_choose(&mut e, 0);
    assert_eq!(charge(&e, choker), 1);
    activate_and_choose(&mut e, 1);
    assert_eq!(charge(&e, choker), 0);
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
    // "Deals damage to you": the Choker's current controller, at that
    // player's own upkeep, never its owner.
    for (controller, recipient, life) in [(p0, p1, 17), (p1, p0, 16)] {
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
        assert_eq!(
            e.state().players[usize::from(recipient.get())].life,
            life,
            "the new controller takes the damage at their upkeep"
        );
        assert_eq!(
            e.state().players[usize::from(controller.get())].life,
            if controller == p0 { 20 } else { 17 },
            "the previous controller takes none"
        );
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
