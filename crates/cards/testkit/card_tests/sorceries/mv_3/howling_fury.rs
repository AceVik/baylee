//! `cards/sorceries/mv_3/howling_fury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Howling Fury prints one line — "Target creature gets +4/+0 until end of
/// turn" — and the board is built so that a single play of it settles every
/// word at once. A 1/1 Elf stands under the caster and another across the
/// table, so the target question has to offer both and the `(5, 1)` that
/// follows has to land on exactly the one that was named: a pump that had
/// lost its target would raise both, and one that read the wrong half of the
/// pair would leave a printed 1/1 a 1/1. `+4/+0` and not `+4/+4` is the other
/// half — a toughness pumped with the power reads `(5, 5)`. The last
/// assertion is the duration: by the following main phase the pump is gone,
/// which is what separates "until end of turn" (CR 514.2) from a counter
/// that would still be sitting there.
#[test]
fn howling_fury_pumps_only_the_creature_it_targets_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[howling_fury()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert_eq!(pt(&engine, theirs), (1, 1), "and one across the table");

    // `cast_from_hand` taps first: three Swamps and the Elves' own `{G}` are
    // four mana, and `{2}{B}` is three of them.
    cast_from_hand(&mut engine, p0, howling_fury());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the target is named first (CR 601.2c) and the cost is the last step \
         of the cast (CR 601.2h), so the whole four are still floating"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{2}}{{B}} comes out of the pool only now, leaving the \
         Elves' one green as the change"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (5, 1),
        "+4/+0 on the creature it named — a (5, 5) would be a toughness the \
         card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );

    // The duration. A pump that never expired would read (5, 1) here too, so
    // only the next turn's main phase can tell the two apart.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"until end of turn\": the cleanup step of the turn it was cast in \
         took the +4/+0 back, so the Elf is the 1/1 it was printed as"
    );
}
