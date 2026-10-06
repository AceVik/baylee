//! `cards/lands/utility/daru_encampment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Daru Encampment is a land printing two abilities: "{T}: Add {C}" and
/// "{W}, {T}: Target Soldier creature gets +1/+1 until end of turn."
///
/// The pump names no controller, so the same Soldier — Serra Zealot, a
/// printed 1/1 Human Soldier — stands on both sides of the table and both must be on the
/// target menu, while the Llanowar Elves beside them and the Encampment itself
/// must not be: that is the whole printed filter read off one question. The {W}
/// comes from a Plains with the land kept back from the tapping, since one
/// {T} pays for both of its abilities, and a turn later the pumped Soldier is
/// the 1/1 it was printed as.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn daru_encampment_pumps_a_soldier_on_either_side_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                daru_encampment(),
                plains(),
                serra_zealot(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[serra_zealot()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let encampment = on_battlefield(&engine, p0, daru_encampment()).expect("the Encampment is out");
    let mine = on_battlefield(&engine, p0, serra_zealot()).expect("my Soldier is out");
    let theirs = on_battlefield(&engine, p1, serra_zealot()).expect("their Soldier is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The Encampment is named as the printing kept back: its own {T} is the
    // price of the ability under test, and `tap_all_mana` would have spent it
    // (#159). What is left floating is one white and one green, which is every
    // mana route on this board — the Elf is a source too.
    tap_all_mana_but(&mut engine, p0, Some(daru_encampment()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "the Plains, one white");
    assert_eq!(pool.total(), 2, "and the Elf's own green beside it");
    assert!(
        !is_tapped(&engine, encampment),
        "the land whose tap the pump charges is still standing"
    );

    // "{{T}}: Add {{C}}" is a printed mana ability, so it is an ordinary
    // `(source, index)` entry; the pump is ability 1, and `can_afford` reads
    // the pool — which is why it is offered only now that the white is in it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(encampment, 0)),
        "the printed mana line is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(encampment, 1)),
        "and so is the pump, with {{W}} floating: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, daru_encampment(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Soldier creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target Soldier creature\" names no controller, so the Soldier \
         across the table is a legal target too: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elf is a creature and no Soldier: {options:?}"
    );
    assert!(
        !options.contains(&encampment),
        "the land is no creature at all, and not a target for its own ability: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while the question stands the mana is still
    // floating and the land is still untapped.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the target is named before the cost is paid"
    );
    assert!(
        !is_tapped(&engine, encampment),
        "and the {{T}} is the last step of the activation, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Soldier the question offered was chosen");

    assert!(is_tapped(&engine, encampment), "{{T}} was paid");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "and the {{W}} with it, out of the pool rather than off the land"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 on the Soldier the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all on the Soldier it did not: one target, one creature"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the non-Soldier never moved either"
    );

    // "until end of turn": a turn later the Soldier is a printed 1/1 again, so
    // the +1/+1 was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, serra_zealot()).is_some(),
        "and the Soldier is still standing, so the pump left rather than the creature"
    );
}
