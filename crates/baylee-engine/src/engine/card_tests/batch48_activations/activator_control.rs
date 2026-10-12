//! Any-player activation distinguishes artifact controller from activator.
#[allow(clippy::wildcard_imports)]
use super::*;

fn clock_upkeep(e: &mut Engine<RegistryLookup>, activator: PlayerId) {
    pass_until(e, |e| {
        e.state.turn.active == P1
            && e.state.turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == activator)
    });
    for player in &mut e.state.players {
        for color in ManaColor::ALL {
            player.mana_pool.add(color, 30);
        }
    }
    e.refresh_offer();
}

#[test]
fn powerleech_checks_activator_not_artifact_controller_for_tapless_abilities() {
    for opponent_activates in [false, true] {
        let mut e = if opponent_activates {
            prepared(&[], &[index::POWERLEECH, index::ARMAGEDDON_CLOCK], &[])
        } else {
            prepared(&[], &[index::POWERLEECH], &[index::ARMAGEDDON_CLOCK])
        };
        let owner = if opponent_activates { P0 } else { P1 };
        let activator = if opponent_activates { P1 } else { P0 };
        clock_upkeep(&mut e, activator);
        let before = e.state.players[0].life;
        let clock = source(&e, owner, index::ARMAGEDDON_CLOCK);
        e.apply(
            activator,
            PlayerAction::ActivateAbility {
                source: clock,
                ability_index: 2,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.players[0].life,
            before + i32::from(opponent_activates)
        );
        assert!(on_battlefield(&e, owner, index::ARMAGEDDON_CLOCK).is_some());
    }
}

#[test]
fn haunting_wind_damages_artifact_controller_not_clock_activator() {
    for owner in [P0, P1] {
        let mut e = if owner == P0 {
            prepared(&[], &[index::HAUNTING_WIND, index::ARMAGEDDON_CLOCK], &[])
        } else {
            prepared(&[], &[index::HAUNTING_WIND], &[index::ARMAGEDDON_CLOCK])
        };
        let activator = if owner == P0 { P1 } else { P0 };
        clock_upkeep(&mut e, activator);
        let before = [e.state.players[0].life, e.state.players[1].life];
        // Any-player activation uses a source controlled by the other seat.
        let clock = source(&e, owner, index::ARMAGEDDON_CLOCK);
        e.apply(
            activator,
            PlayerAction::ActivateAbility {
                source: clock,
                ability_index: 2,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.players[usize::from(owner.get())].life,
            before[usize::from(owner.get())] - 1
        );
        assert_eq!(
            e.state.players[usize::from(activator.get())].life,
            before[usize::from(activator.get())]
        );
    }
}
