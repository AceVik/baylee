//! `cards/creatures/mv_2/hapatra_vizier_of_poisons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The target is mandatory when the combat trigger is stacked; placing the
/// counter is optional only as it resolves (CR 603.3d, 608.2d).
#[allow(clippy::too_many_lines)] // a combat played to damage, which is where the trigger lives
#[test]
fn hapatra_marks_the_creature_she_points_at_when_she_connects() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[hapatra_vizier_of_poisons(), llanowar_elves()])
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: which seat the seed
    // puts on the play decides whether a whole turn is in the way, and that
    // turn's combat is a question `reach_main_phase` has no arm for.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let hapatra =
        on_battlefield(&engine, p0, hapatra_vizier_of_poisons()).expect("Hapatra is on the table");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("your own Elves are out");
    let victim =
        on_battlefield(&engine, p1, sheoldred_the_apocalypse()).expect("the Praetor is out");
    assert_eq!(pt(&engine, hapatra), (2, 2), "the body as printed");
    assert_eq!(
        counters_on(&engine, victim, CounterKind::M1M1),
        0,
        "nothing has marked anybody yet"
    );

    // Into combat. The attack is declared by hand, because `pass_until`'s own
    // answer to this question is an empty one — and a combat that never
    // happens is a trigger that never fires.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0);
    assert!(
        attackers.contains(&hapatra),
        "a permanent that stood on the battlefield before the turn began is \
         not summoning sick: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "the only thing to attack is the opponent"
    );
    let defender = defenders
        .into_iter()
        .next()
        .expect("one defender, checked above");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(hapatra, defender)],
            },
        )
        .unwrap();

    // p1 may block with a 4/5 and chooses not to, so Hapatra connects and the
    // printed trigger asks what it may mark.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the controller of the trigger chooses");
    assert_eq!(
        (min, max),
        (1, 1),
        "the trigger chooses exactly one target; the counter is optional later"
    );
    assert!(
        options.contains(&victim) && options.contains(&mine),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, victim, CounterKind::M1M1),
        1,
        "one -1/-1 counter, and not a +1/+1"
    );
    assert_eq!(
        pt(&engine, victim),
        (3, 4),
        "the counter read back through the layer system: a 4/5 marked once"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and only the creature that was named took anything"
    );
    assert_eq!(
        pt(&engine, hapatra),
        (2, 2),
        "Hapatra marks, she is not marked"
    );
}

/// "you **may** put a -1/-1 counter on target creature", and "whenever you
/// put one or more -1/-1 counters on a creature, create a 1/1 Snake". The
/// target is named either way (CR 603.3d); answering the "may" with no puts no
/// counter on it and, because no counter was placed, makes no Snake. The
/// accepting run on the same board is the control that shows both happen.
#[allow(clippy::too_many_lines)] // two combats played to damage, which is where the trigger lives
#[test]
fn hapatra_declining_the_may_places_no_counter_and_makes_no_snake() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for accept in [true, false] {
        let mut engine = Duel::new(97, forest())
            .battlefield(0, &[hapatra_vizier_of_poisons(), llanowar_elves()])
            .battlefield(1, &[sheoldred_the_apocalypse()])
            .start();
        keep_mulligans(&mut engine);
        assert!(
            walk_to_own_main(&mut engine, p0),
            "p0 reaches its own first main phase"
        );
        let hapatra = on_battlefield(&engine, p0, hapatra_vizier_of_poisons()).unwrap();
        let mine = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
        let victim = on_battlefield(&engine, p1, sheoldred_the_apocalypse()).unwrap();
        let permanents = engine.state().zones.list(ZoneLocation::Battlefield).len();

        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
            unreachable!("the predicate just matched")
        };
        let defender = defenders.into_iter().next().expect("one defender");
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(hapatra, defender)],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![victim],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        engine.apply(p0, PlayerAction::YesNo(accept)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            counters_on(&engine, victim, CounterKind::M1M1),
            u16::from(accept),
            "the -1/-1 counter is placed only when the \"may\" is accepted (accept = {accept})"
        );
        assert_eq!(
            pt(&engine, victim),
            if accept { (3, 4) } else { (4, 5) },
            "and the victim is marked or untouched accordingly (accept = {accept})"
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Battlefield).len(),
            permanents + usize::from(accept),
            "a Snake token appears only because a -1/-1 counter was placed (accept = {accept})"
        );
        assert_eq!(
            counters_on(&engine, mine, CounterKind::M1M1),
            0,
            "the counter, or its absence, is the named target's alone (accept = {accept})"
        );
    }
}
