//! `cards/creatures/mv_2/priest_of_titania.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Priest of Titania — {1}{G}, 1/1 Elf Druid: "{T}: Add {G} for each Elf on
/// the battlefield."
///
/// The count is the whole card, and the word that needs a witness is "on the
/// battlefield": the Priest is an Elf itself, two Llanowar Elves stand beside
/// it and a fourth stands across the table, with no "you control" anywhere in
/// the sentence to keep that one out. Only the two Elves of mine are tapped
/// for mana — the Priest is named as the thing kept back, because its own
/// `{T}` is exactly what this test presses — so the four green that arrive are
/// one per Elf in play and can be read as neither two nor three.
/// The two Shadows stand on the same side of the table as the light: the
/// `{T}` is the whole price of the ability and nothing is asked on the way
/// (CR 605.3b), so the mana is in the pool the moment it is activated.
#[test]
fn priest_of_titania_taps_for_one_green_per_elf_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[priest_of_titania(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let priest = on_battlefield(&engine, p0, priest_of_titania()).expect("the Priest is out");
    assert_eq!(pt(&engine, priest), (1, 1), "a printed 1/1");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "an Elf across the table, which \"on the battlefield\" counts"
    );

    // Two Elves and only those: the Priest prints the very `{T}` this test
    // presses, so `tap_all_mana` would have spent it (#159).
    tap_all_mana_but(&mut engine, p0, Some(priest_of_titania()));
    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);
    assert_eq!(before, 2, "two Elves tapped, two green floating");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(priest, 0)),
        "the whole price is an untapped Priest's own {{T}}, so its one line is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, priest_of_titania(), 0);

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the green is already here"
    );
    assert!(is_tapped(&engine, priest), "the Priest paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        6,
        "the two green the Elves made plus one for each of the four Elves on \
         the battlefield: the Priest itself, my two, and the one across the \
         table"
    );
    assert_eq!(
        pool.total(),
        6,
        "and nothing else came with it — no other colour, no other source"
    );
}
