#[allow(clippy::wildcard_imports)]
use super::*;

fn decline_entry(e: &mut Engine<RegistryLookup>, permanent: ObjectId) {
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
    targets(e, vec![permanent], vec![]);
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn batch47_derevi_command_activation_ignores_existing_tax_and_is_not_a_cast() {
    let mut e = Duel::new(SEED, index::FOREST)
        .commander(0, &[index::DEREVI_EMPYRIAL_TACTICIAN])
        .battlefield(0, &[index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let derevi = e.state.zones.list(ZoneLocation::Command(P0))[0];
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    e.state.commander_casts[0] = 4;
    e.state.commanders[0][0].casts = 4;
    assert_eq!(crate::casting::commander_tax(e.state(), P0, derevi), 8);
    set_pool(
        &mut e,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        !legal.castable.contains(&derevi),
        "eight additional commander-tax mana are unavailable"
    );
    activate_as(&mut e, P0, derevi, 2);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert_eq!(
        e.state.object(derevi).unwrap().zone,
        Zone::Command,
        "activation uses the stack"
    );
    decline_entry(&mut e, forest);
    assert_eq!(e.state.object(derevi).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.commander_casts[0], 4);
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: derevi,
                ability_index: 2
            }
        )
        .is_err()
    );
}

#[test]
fn batch47_derevi_targets_before_choosing_tap_or_untap_on_resolution() {
    for mode in [0, 1] {
        let mut e = prepared(&[index::DEREVI_EMPYRIAL_TACTICIAN], &[index::FOREST], &[]);
        let derevi = in_hand(&e, P0, index::DEREVI_EMPYRIAL_TACTICIAN).unwrap();
        let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
        e.apply(P0, PlayerAction::CastSpell { card: derevi })
            .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        targets(&mut e, vec![forest], vec![]);
        // Responding after target selection can change the relevant tap state.
        e.state.set_tapped(forest, true);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        assert!(matches!(e.pending(), Pending::ChooseCastMode { .. }));
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(is_tapped(&e, forest), mode == 0);
    }
}

#[test]
fn batch47_derevi_command_activation_tracks_the_exact_source_incarnation() {
    let mut e = Duel::new(SEED, index::FOREST)
        .commander(0, &[index::DEREVI_EMPYRIAL_TACTICIAN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let derevi = e.state.zones.list(ZoneLocation::Command(P0))[0];
    set_pool(
        &mut e,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    activate_as(&mut e, P0, derevi, 2);
    e.state
        .move_object(
            derevi,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            derevi,
            ZoneLocation::Command(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(derevi).unwrap().zone, Zone::Command);
    assert_eq!(e.state.commander_casts[0], 0);
}
