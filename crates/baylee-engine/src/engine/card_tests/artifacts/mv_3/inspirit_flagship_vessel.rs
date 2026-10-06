//! `cards/artifacts/mv_3/inspirit_flagship_vessel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Station (CR 702.184a): "Tap another untapped creature you control: Put a
/// number of charge counters on this permanent equal to the tapped
/// creature's power." The Raptor is paid, not targeted, so the ability on
/// the stack names nothing, and it is still on the battlefield as the
/// ability resolves, so the count is its power there, counter and all
/// (CR 608.2h). Evendo's tests read the other half of that rule, a creature
/// gone by then.
#[test]
fn stationing_a_creature_reads_the_power_it_still_has() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(43, plains(), &[inspirit_flagship_vessel()]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the ship is out");

    station_inspirit(&mut engine, bird);
    assert!(is_tapped(&engine, bird), "the cost is paid");
    let ability = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("station is on the stack");
    assert!(
        engine
            .state()
            .object(ability)
            .is_some_and(|o| o.targets.is_empty()),
        "station targets nothing"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(vessel)
            .map(|o| o.counters.get(CounterKind::Charge)),
        Some(2),
        "the Raptor's power on the battlefield, counter and all",
    );
}

/// "Tap another **untapped** creature you control" (CR 702.184a): a
/// creature already tapped cannot pay the cost (CR 118.3), so station never
/// offers it. Inspirit wrote station as an effect, `Effect::TapTarget` under
/// a free cost aimed at "another creature you control", so a tapped Elf was
/// a legal target, and "tapping" it again put its power in charge counters
/// for nothing.
#[test]
fn inspirit_stations_only_off_an_untapped_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                inspirit_flagship_vessel(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    let [tapped, untapped] = all_on_battlefield(&engine, p0, llanowar_elves())[..] else {
        panic!("two Elves");
    };
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .set_tapped(tapped, true);

    activate(&mut engine, p0, inspirit_flagship_vessel(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "station asks which untapped creature pays, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, ChoicePrompt::CostTap, "a cost, not a target");
    assert_eq!(options, vec![untapped], "only the untapped Elf can pay");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![tapped],
                },
            )
            .is_err(),
        "the tapped Elf is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![untapped],
            },
        )
        .expect("the untapped Elf taps");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, vessel, CounterKind::Charge),
        1,
        "one Elf's power"
    );
}

/// A Spacecraft that stations to 8+ becomes a 5/5, not a corpse.
///
/// "It's an artifact creature at 8+" turns the type on, and the card def
/// carried no power or toughness at all — so the Vessel became a creature
/// with no body and the next state-based check put it into the graveyard.
/// A Spacecraft prints its numbers exactly as a Vehicle does and uses them
/// only once it is stationed.
#[test]
fn a_stationed_spacecraft_becomes_the_creature_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, island())
        .battlefield(0, &[inspirit_flagship_vessel()])
        .start();
    keep_mulligans(&mut engine);
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    assert!(
        !engine
            .state()
            .object(vessel)
            .expect("the Vessel is an object")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "an unstationed Spacecraft is no creature",
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, vessel, CounterKind::Charge, 8);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert!(
        on_battlefield(&engine, p0, inspirit_flagship_vessel()).is_some(),
        "a stationed Spacecraft is still on the battlefield",
    );
    let chars = engine
        .state()
        .object(vessel)
        .expect("the Vessel is an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::CREATURE),
        "at 8+ it is an artifact creature",
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(5), Some(5)),
        "and the body it prints is the body it gets",
    );
}

/// The owner's table (report 01a0e3ea, #322): Inspirit, Flagship Vessel
/// cast beside two artifacts, no charge counter on it yet.
///
/// "8+ | Flying / Other artifacts you control have hexproof and
/// indestructible." Both sentences are printed in the 8+ striation, so both
/// are part of that station symbol's static ability (CR 721.2): "As long as
/// this permanent has N or more charge counters on it, it has [abilities]"
/// (CR 721.2a). The engine granted the pair from the moment the Vessel
/// landed — the report's own view has both of the owner's Roaming Thrones
/// hexproof and indestructible under an Inspirit with no counters.
///
/// Seven counters are placed by hand and the eighth by stationing a 1/1,
/// so the threshold is crossed by the Vessel's own ability; one counter
/// taken off again is the grant lapsing.
#[test]
fn inspirit_protects_other_artifacts_only_at_eight_charge_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, island())
        .battlefield(
            0,
            &[
                inspirit_flagship_vessel(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    let other = on_battlefield(&engine, p0, quiet_artifact()).expect("the other artifact");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    assert!(
        !hexproof_and_indestructible(&engine, other),
        "no charge counter, no 8+ striation: the other artifact is unprotected"
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, vessel, CounterKind::Charge, 7);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == vessel)
        .expect("the station ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    assert!(
        !hexproof_and_indestructible(&engine, other),
        "seven is not eight"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(vessel)
            .map(|o| o.counters.get(CounterKind::Charge)),
        Some(8),
        "the Elves' power made the eighth"
    );
    assert!(
        hexproof_and_indestructible(&engine, other),
        "at 8+ the other artifact has hexproof and indestructible"
    );
    assert!(
        !hexproof_and_indestructible(&engine, vessel),
        "and the Vessel itself does not: \"other artifacts\""
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::remove_counters(state, vessel, CounterKind::Charge, 1);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        !hexproof_and_indestructible(&engine, other),
        "below eight again, the grant is gone"
    );
}

/// Inspirit's 1+ striation: "At the beginning of combat on your turn, put
/// your choice of a +1/+1 counter or two charge counters on up to one other
/// target artifact."
///
/// The Vessel has that ability only while it has a charge counter
/// (CR 721.2a), so with none it does not trigger at all — the engine used to
/// ask for its mode on the Vessel's first combat. With one it triggers, and
/// once it has triggered it no longer depends on its source (CR 113.7a):
/// losing the counter in response does not take it off the stack, which is
/// what an intervening `if` (CR 603.4) would have done.
#[test]
fn inspirit_combat_trigger_needs_a_charge_counter_and_outlives_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, island())
        .battlefield(0, &[inspirit_flagship_vessel(), quiet_artifact(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    let other = on_battlefield(&engine, p0, quiet_artifact()).expect("the other artifact");
    let vessel_triggers = |engine: &Engine<RegistryLookup>| {
        engine
            .journal()
            .entries()
            .iter()
            .filter(|e| {
                matches!(e.event, crate::event::GameEvent::AbilityTriggered { source, .. }
                    if source == vessel)
            })
            .count()
    };

    // No counter: through combat without a question (`pass_until` refuses
    // a mode question it was not told about).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(vessel_triggers(&engine), 0, "no charge counter, no trigger");

    // One counter: the next combat of p0's asks.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, vessel, CounterKind::Charge, 1);
    assert!(walk_to_own_main(&mut engine, p0), "p0's next main");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("standing on the mode question")
    };
    let charge = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("two charge counters is offered");
    engine.apply(p0, PlayerAction::ChooseMode(charge)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the trigger's target, got {:?}", engine.pending())
    };
    assert!(options.contains(&other), "the other artifact: {options:?}");
    assert!(!options.contains(&vessel), "\"other target artifact\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![other],
                players: vec![],
            },
        )
        .unwrap();
    assert!(!stack_is_empty(&engine), "the trigger is on the stack");
    assert_eq!(
        vessel_triggers(&engine),
        1,
        "one charge counter, one trigger"
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::remove_counters(state, vessel, CounterKind::Charge, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(other)
            .map(|o| o.counters.get(CounterKind::Charge)),
        Some(2),
        "the trigger resolved although the Vessel lost its counter"
    );
}
