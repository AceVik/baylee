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
