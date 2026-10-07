//! `cards/lands/planets/evendo_waking_haven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Evendo, Waking Haven prints `This land enters tapped.`, `{{T}}: Add {{G}}.`,
/// Station, and `12+ | {{G}}, {{T}}: Add {{G}} for each creature you control.`
///
/// This test plays the land to confirm it enters tapped, passes the turn to
/// untap it, verifies that with no other creature to tap and no charge
/// counters only ability 0 is offered, and taps it for `{G}`.
#[test]
fn evendo_waking_haven_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[evendo_waking_haven()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, evendo_waking_haven());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        1,
        "only ability 0 is offered because charge counter threshold is not met"
    );

    activate(&mut engine, p0, evendo_waking_haven(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Evendo's station (CR 702.184a): "Tap another untapped creature you
/// control: Put a number of charge counters on this permanent equal to the
/// tapped creature's power." A Llanowar Elves with two +1/+1 counters is a
/// 3/3, and three charge counters is what it puts there. The Elves is paid,
/// not targeted: the ability on the stack names no target.
#[test]
fn evendo_stations_off_the_tapped_creatures_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[evendo_waking_haven(), llanowar_elves(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    let evendo = on_battlefield(&engine, p0, evendo_waking_haven()).expect("Evendo");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    crate::replacement::put_counters(
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up"),
        elves[0],
        CounterKind::P1P1,
        2,
    );
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, elves[0]), (3, 3));

    station_evendo(&mut engine, elves[0]);
    assert!(is_tapped(&engine, elves[0]), "the cost is paid");
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
        counters_on(&engine, evendo, CounterKind::Charge),
        3,
        "the tapped creature's power"
    );
    assert!(!is_tapped(&engine, elves[1]), "one creature, one tap");
}

/// The tapped creature's power is read as station resolves, and a creature
/// that has left by then counts as it last existed on the battlefield
/// (CR 608.2h): the Elves tapped for station and Lightning Bolted in
/// response still puts one counter on Evendo.
#[test]
fn evendo_counts_a_creature_killed_in_response_as_it_last_was() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[evendo_waking_haven(), llanowar_elves()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let evendo = on_battlefield(&engine, p0, evendo_waking_haven()).expect("Evendo");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");

    station_evendo(&mut engine, elves);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).expect("the Bolt");
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .expect("a Mountain pays for the Bolt");
    aim_at(&mut engine, p1, elves);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Elves died first"
    );
    assert_eq!(
        counters_on(&engine, evendo, CounterKind::Charge),
        1,
        "its power as it last existed on the battlefield"
    );
}

/// Evendo at 12+ (CR 721.2a): "{G}, {T}: Add {G} for each creature you
/// control." At eleven charge counters the ability is not there; a station
/// off a 1/1 makes twelve, and then {G} and a tap make one {G} for each of
/// the three Elves.
#[test]
fn evendo_at_twelve_adds_green_for_each_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                evendo_waking_haven(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let evendo = on_battlefield(&engine, p0, evendo_waking_haven()).expect("Evendo");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    crate::replacement::put_counters(
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up"),
        evendo,
        CounterKind::Charge,
        11,
    );
    reach_main_phase(&mut engine, p0);
    let offered = |engine: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal.abilities.contains(&(evendo, 2))
    };
    // The {G} floats first: the offer names an ability whose mana is
    // already in the pool, so without it neither answer below would say
    // anything about the counters.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("the Forest taps for {G}");
    assert!(!offered(&engine), "eleven is not 12+");

    station_evendo(&mut engine, elves[0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, evendo, CounterKind::Charge), 12);
    assert!(offered(&engine), "12+");

    activate(&mut engine, p0, evendo_waking_haven(), 2);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "{{G}} paid, and one {{G}} for each of three creatures"
    );
    assert!(is_tapped(&engine, evendo));
}
