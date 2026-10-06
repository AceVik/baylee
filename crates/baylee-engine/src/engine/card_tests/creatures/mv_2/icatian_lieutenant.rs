//! `cards/creatures/mv_2/icatian_lieutenant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Icatian Lieutenant is `{W}{W}` for a 1/2 Human **Soldier** with
/// `{1}{W}: Target Soldier creature gets +1/+0 until end of turn`. The word
/// "Soldier" is the line's only filter, so next to the Lieutenant stands
/// a Llanowar Elves (Elf Druid, not a Soldier), whom the offer must not name,
/// and across the table a second Lieutenant, who — unlike with
/// "you control" — very much is one. The +1/+0 lands on the Soldier
/// that was named: the 1/2 becomes a 2/2, and the second Soldier and the
/// Elf remain what they were.
#[test]
fn icatian_lieutenant_pumps_the_soldier_it_names_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[icatian_lieutenant()])
        .battlefield(1, &[icatian_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {W}{W} from two of the four Plains; the ability then still wants
    // {1}{W}, and the mana stays in the pool until end of step (CR 500.5),
    // because this scenario never leaves the main phase.
    cast_from_hand(&mut engine, p0, icatian_lieutenant());
    pass_until(&mut engine, stack_is_empty);
    let lieutenant = on_battlefield(&engine, p0, icatian_lieutenant()).expect("der Leutnant liegt");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("der Elf steht");
    let theirs = on_battlefield(&engine, p1, icatian_lieutenant()).expect("ihr Leutnant steht");
    assert_eq!(pt(&engine, lieutenant), (1, 2), "der gedruckte 1/2");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "vier Plains und der Elf sind fünf Mana, zwei davon für {{W}}{{W}}"
    );

    activate(&mut engine, p0, icatian_lieutenant(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Soldier creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "der aktivierende Sitz wählt");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&lieutenant),
        "der Leutnant ist selbst ein Soldier: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"Soldier\" ist der ganze Filter und kein \"you control\": {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "der Llanowar Elves ist ein Elf Druid und kein Soldat: {options:?}"
    );
    assert_eq!(options.len(), 2, "und die beiden sind das ganze Menü");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "CR 601.2h pays last: while the question is pending, no mana is spent"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .expect("der Leutnant war eine der Optionen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{W}} are paid from the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, lieutenant),
        (2, 2),
        "der genannte Soldat bekommt +1/+0"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 2),
        "the Soldier across the table was not named and does not grow"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a non-Soldier cannot be named at all"
    );
}
