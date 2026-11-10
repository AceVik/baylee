//! `cards/creatures/mv_5/briarknit_kami.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Briarknit Kami — {3}{G}{G} 3/3 Spirit: "Whenever you cast a Spirit or
/// Arcane spell, put a +1/+1 counter on target creature."
///
/// Three casts on one board, each reading a different word. The Kami's own
/// cast asks for nothing: it is a Spirit, but its ability functions only
/// once it is on the battlefield (CR 113.6), so the spell that puts it there
/// is not one it sees. Child of Thorns — a {G} Spirit — is the cast that
/// triggers, and the counter lands on an Elf already on the table. An Elf
/// creature spell cast last asks for nothing at all, so what is read is the
/// subtype filter and not merely the fact that a creature spell happened.
#[test]
fn briarknit_kami_marks_a_creature_for_a_spirit_cast_and_not_for_its_own_or_an_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(431, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[briarknit_kami(), child_of_thorns(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before anything");

    // Seven Forests, and the Elf named as the printing kept back: it is the
    // creature the trigger is about to be aimed at, and its own
    // `{T}: Add {G}` is a mana route `tap_all_mana` would otherwise have
    // taken (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Forests, and the Elf contributed nothing"
    );

    cast_with_floating(&mut engine, p0, briarknit_kami());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the Kami is a Spirit spell, but its ability is not on the battlefield \
         yet, so its own cast triggers nothing: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    let kami = on_battlefield(&engine, p0, briarknit_kami()).expect("the Kami resolved");
    assert_eq!(
        pt(&engine, kami),
        (3, 3),
        "and it arrives as the printed 3/3"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{3}}{{G}}{{G}} out of seven leaves the {{G}} for each spell below"
    );

    // The trigger asks while Child of Thorns is still a spell, so the
    // creatures it can name are the ones already on the board.
    cast_with_floating(&mut engine, p0, child_of_thorns());
    let options = options_offered_including(&mut engine, elf);
    assert!(
        options.contains(&elf) && options.contains(&kami),
        "\"target creature\" reaches the board the Spirit has not landed on \
         yet: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "\"put a +1/+1 counter on target creature\" — a counter, not a pump"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "one +1/+1 on a printed 1/1");
    assert!(
        on_battlefield(&engine, p0, child_of_thorns()).is_some(),
        "the Spirit that triggered it resolved as well"
    );

    // The control: the other creature spell in hand is an Elf, so the only
    // thing that differs from the cast above is the subtype the printed
    // sentence names.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "an Elf is a creature spell and neither a Spirit nor an Arcane one, \
         so nothing triggers and no target is asked for: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "the Elf spell put no counter on anything"
    );
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "the second Elf resolved, so the cast that asked nothing really happened"
    );
}
