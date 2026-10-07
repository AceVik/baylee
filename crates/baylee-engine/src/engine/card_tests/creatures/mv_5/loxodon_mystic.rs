//! `cards/creatures/mv_5/loxodon_mystic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Loxodon Mystic is a printed 3/3 for {3}{W}{W} with one line of text:
/// "{W}, {T}: Tap target creature." Both halves of that price are the engine's
/// answer rather than the card's, so the board reads them in the rules' order —
/// with the target question still open (CR 601.2c) the Mystic is untapped and
/// the white is in the pool, and only the answer to it (CR 601.2h) spends both.
/// "Target creature" names the whole table and not one seat of it, so an Elf
/// beside the Mystic and two Elves across it are all on the menu while only the
/// creature actually named comes back tapped.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn loxodon_mystic_taps_and_spends_a_white_to_tap_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                loxodon_mystic(),
                llanowar_elves(),
            ],
        )
        // Two Elves across the table, so the menu has to choose between two
        // creatures that are equally legal and the one left alone afterwards
        // is a bystander of the same card.
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mystic = on_battlefield(&engine, p0, loxodon_mystic()).expect("the Mystic is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves across the table");
    let (victim, bystander) = (theirs[0], theirs[1]);
    assert_eq!(pt(&engine, mystic), (3, 3), "the body the card prints");
    assert!(!is_tapped(&engine, mystic), "and it is standing untapped");

    // Six mana is not needed and three Plains are enough: the Mystic is
    // seated rather than cast, so only the ability's own {W} has to be paid.
    // Both creatures are named as the printing kept back — the Mystic's own
    // {T} is the price under test, and a mana creature tapped for the cost
    // would put its own {G} in the pool beside the white.
    tap_mana_where(&mut engine, p0, |id| id != mystic && id != mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Plains, three white, and neither creature contributed"
    );
    assert!(
        !is_tapped(&engine, mystic) && !is_tapped(&engine, mine),
        "the two creatures were the sources kept back from the tapping"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands — so the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mystic, 0)),
        "with {{W}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, loxodon_mystic(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&mine),
        "\"target creature\" names either side of the table: my own Elf is on \
         the menu: {options:?}"
    );
    assert!(
        options.contains(&victim) && options.contains(&bystander),
        "and so are both Elves across it: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // halves of the price are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, mystic),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{W}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        is_tapped(&engine, mystic),
        "{{T}} is paid by the Mystic itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{W}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a creature is no mana ability, so the ability is on the stack"
    );
    assert!(
        !is_tapped(&engine, victim),
        "and nothing has happened to the target yet: the tap is the resolution"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, victim),
        "\"Tap target creature\" — the Elf the ability named is tapped"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "the Elf nobody named never moved: the effect targets one creature, it \
         does not sweep the board"
    );
    assert!(
        !is_tapped(&engine, mine),
        "nor does it reach the creature on its own side that it could have named"
    );
    assert!(
        on_battlefield(&engine, p0, loxodon_mystic()).is_some(),
        "the price was a tap and a white, so the Mystic is still standing"
    );
}
