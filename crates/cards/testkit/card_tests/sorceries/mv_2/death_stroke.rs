//! `cards/sorceries/mv_2/death_stroke.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Death Stroke is `{B}{B}` for "Destroy target tapped creature", and the whole
/// card is the word "tapped": the filter is `CREATURE` **and** `Tapped`, so a
/// board that holds both readings at once is what separates them. Two Llanowar
/// Elves — the same card, the same 1/1 body and even the same mana ability —
/// stand one on each side of the table, and only this seat's is tapped, for its
/// own mana. The `{B}{B}` is floated and the board read *before* the cast, so a
/// Death Stroke that could point at any creature would be castable a step too
/// early; afterwards the untapped Elf is the counter-half of the offer and the
/// tapped one is what has to be in a graveyard.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn death_stroke_destroys_the_tapped_creature_and_no_untapped_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[death_stroke()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is on the table");
    let their_elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is on the table");
    let spell = in_hand(&engine, p0, death_stroke()).expect("Death Stroke is in hand");

    // The two Swamps alone, with the Elf named as the source to keep back: two
    // black is exactly the printed {B}{B}, so the only thing this board is
    // missing is a creature that is already tapped.
    tap_mana_except(&mut engine, p0, elf);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps tapped, and the Elf left standing"
    );
    assert!(
        !is_tapped(&engine, elf) && !is_tapped(&engine, their_elf),
        "no creature in the game is tapped yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "{{B}}{{B}} is paid for and there is still nowhere to point it: every \
         creature in the game is untapped, so the spell is not offered: {:?}",
        legal.castable
    );

    // Tapping the Elf for its own {G} is what puts a tapped creature on the
    // board; the green it makes is a side effect of the price it pays.
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert!(
        is_tapped(&engine, elf),
        "a mana ability's whole price is its own {{T}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again: {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "with a tapped creature to point at, the spell is offered: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, death_stroke());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target tapped creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&elf),
        "the tapped creature is the whole of the filter: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "\"tapped\" is read: the creature across the table is the same card \
         and still no legal target while it stands up: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and my Elf is the only tapped creature in the game: {options:?}"
    );

    // CR 601.2c before CR 601.2h: the target is named while the mana is still
    // in the pool and the creature is still on the battlefield.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "the {{B}}{{B}} is spent only once the target has been named"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered is the one it dies to");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and a destroyed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell declined never moved"
    );
    assert!(
        in_graveyard(&engine, p0, death_stroke()).is_some(),
        "the sorcery itself is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{B}}{{B}} was paid, and the Elf's own {{G}} is all that is left"
    );
}
