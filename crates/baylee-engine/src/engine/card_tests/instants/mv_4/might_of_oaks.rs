//! `cards/instants/mv_4/might_of_oaks.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Might of Oaks prints one sentence — "Target creature gets +7/+7 until end
/// of turn" — and every word of it needs a witness: "target" is a single
/// choice out of a menu that reaches both sides of the table, "creature" is
/// what tells the pump from a board-wide grant, and "until end of turn" is a
/// duration nothing on the battlefield carries. The Elf across the table and
/// the turn walked afterwards are those witnesses: both printed numbers land
/// on the creature that was named and on no other, and they are gone again by
/// the next main phase while the creature is still standing.
#[test]
fn might_of_oaks_pumps_the_creature_it_names_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[might_of_oaks()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the pump");

    // Five Forests tapped and the Elves named as the printing kept back: the
    // creature this test targets is one of them, and its own `{T}` would have
    // been spent for mana the {3}{G} does not need.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped, five green, and neither Elf paid in"
    );
    cast_with_floating(&mut engine, p0, might_of_oaks());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (8, 8),
        "+7/+7 on the creature the spell named, and on nothing else"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf the spell did not name never moved — \"target creature\" is \
         not \"creatures\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}}{{G}} came out of the five green: one mana is the change left"
    );

    // "until end of turn": the pump is a duration and not a static, so the
    // same Elf reads its printed body a turn later.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the numbers left rather than \
         the creature"
    );
}
