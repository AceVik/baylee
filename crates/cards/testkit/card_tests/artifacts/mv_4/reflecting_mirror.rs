//! `cards/artifacts/mv_4/reflecting_mirror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn reflecting_mirror() -> CardIndex {
    card_index("2fb505c6-3d28-4b83-9820-4b9db64c8304")
}

fn bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// p1 Bolts either p0 (`at_p0`) or p0's Bears on p1's own turn, and passes:
/// the Bolt is on the stack and p0 holds priority with the Mirror and two
/// untapped Forests. Returns the engine and the Bolt.
fn a_bolt_in_flight(at_p0: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[reflecting_mirror(), forest(), forest(), grizzly_bears()],
        )
        .battlefield(1, &[mountain()])
        .hand(1, &[bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, bolt());
    let (objects, players) = if at_p0 {
        (vec![], vec![p0])
    } else {
        (vec![bears], vec![])
    };
    engine
        .apply(p1, PlayerAction::ChooseTargets { objects, players })
        .expect("the Bolt's target was on the offer");
    let spell = on_stack(&engine, bolt()).expect("the Bolt is on the stack");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "p0 may respond: {:?}",
        engine.pending()
    );
    (engine, spell, bears)
}

/// Activates the Mirror with X = 2 (twice the Bolt's mana value 1) at the
/// Bolt and pays for it with the Forests; the ability ends on the stack.
#[track_caller]
fn mirror_at(engine: &mut Engine<RegistryLookup>, spell: ObjectId) {
    let p0 = PlayerId::new(0);
    activate(engine, p0, reflecting_mirror(), 0);
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseNumber { max, .. } => {
                assert!(max >= 2, "two Forests stand untapped, got max {max}");
                engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { options, .. } => {
                assert!(
                    options.contains(&spell),
                    "the Bolt (mana value 1, X = 2) is on the menu: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![spell],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } if engine.payment_window().is_some() => {
                tap_all_mana(engine, p0);
                engine.apply(p0, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while activating the Mirror: {other:?}"),
        }
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "the Bolt and, above it, the Mirror's ability"
    );
}

/// Reflecting Mirror (2fb505c6-3d28-4b83-9820-4b9db64c8304) — "{X}, {T}: Change
/// the target of target spell with a single target if that target is you. The
/// new target must be a player. X is twice the mana value of that spell."
///
/// p1's Bolt (mana value 1) aims at p0; the Mirror with X = 2 sends it to the
/// only other player, p1, who takes the 3.
#[test]
fn reflecting_mirror_turns_a_bolt_at_you_back_on_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, spell, _bears) = a_bolt_in_flight(true);
    let mirror = on_battlefield(&engine, p0, reflecting_mirror()).expect("the Mirror");
    mirror_at(&mut engine, spell);
    assert!(is_tapped(&engine, mirror), "{{T}} was paid");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "X = 2 was spent"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the new-target question")
    };
    assert_eq!(player, p0, "the Mirror's controller picks the new target");
    assert!(options.is_empty(), "the new target must be a player");
    assert!(player_options.contains(&p1), "the caster is a candidate");
    assert!(!player_options.contains(&p0), "and the old target is not");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 was on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 20, "p0 took nothing");
    assert_eq!(life(&engine, p1), 17, "the Bolt hit its caster");
}

/// "If that target is you": a Bolt aimed at the Bears is left alone, the
/// Mirror's ability does nothing, and the Bears die.
#[test]
fn reflecting_mirror_leaves_a_spell_aimed_elsewhere_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, spell, _bears) = a_bolt_in_flight(false);
    mirror_at(&mut engine, spell);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "the Bolt still hit the Bears"
    );
    assert_eq!(life(&engine, p1), 20);
    assert_eq!(life(&engine, p0), 20);
}
