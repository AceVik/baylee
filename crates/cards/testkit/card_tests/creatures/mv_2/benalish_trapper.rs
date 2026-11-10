//! `cards/creatures/mv_2/benalish_trapper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Benalish Trapper prints one line — "{W}, {T}: Tap target creature." — and
/// two words of it need a witness on the same board. "Target **creature**" is
/// the whole table and not just its controller's side, so the Elf across the
/// table must be offered while the Plains beside the Trapper, a permanent and
/// no creature, must not. "**Tap**" is the entire effect, which is why the Elf
/// standing next to the Trapper is still upright when the dust settles — a
/// tap that caught the whole board would read exactly like a correct one if
/// the only creature in play were the target.
#[test]
fn benalish_trapper_taps_the_creature_it_targets_and_leaves_the_rest_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[plains(), plains(), benalish_trapper(), quiet_creature()],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let trapper = on_battlefield(&engine, p0, benalish_trapper()).expect("the Trapper is out");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elves are out");
    let land = on_battlefield(&engine, p0, plains()).expect("a Plains is out");
    assert!(!is_tapped(&engine, trapper), "nothing is tapped yet");

    // The Elves are kept standing: the creature beside the Trapper is the
    // half of "and only that one" that an empty board could not show, and a
    // creature tapped for mana is a creature whose status moved for a reason
    // of its own.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "two Plains tapped for two white — {{W}} for the ability and one over"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the Elves beside them made nothing: it is this test's control"
    );

    activate(&mut engine, p0, benalish_trapper(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    assert!(
        options.contains(&trapper),
        "the Trapper is itself a creature in play: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Plains is a permanent and no creature: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "and the three creatures in play are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    // CR 601.2c chooses the target first and CR 601.2h pays last, so the tap
    // symbol and the {W} are spent only now, on the far side of the answer.
    assert!(is_tapped(&engine, trapper), "{{T}} is paid for the ability");
    assert!(
        !is_tapped(&engine, mine),
        "and no other permanent on the board has moved yet"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one of the two white was the price; the other is still floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a creature is no mana ability, so the effect waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, theirs),
        "the creature the ability was aimed at is tapped"
    );
    assert!(
        !is_tapped(&engine, mine),
        "and the creature beside it is untouched: the effect is one target, \
         not the board"
    );
}
