//! `cards/creatures/mv_6/progenitor_mimic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Progenitor Mimic — {4}{G}{U} 0/0 Shapeshifter: "You may have this creature
/// enter as a copy of any creature on the battlefield, except it has 'At the
/// beginning of your upkeep, if this creature isn't a token, create a token
/// that's a copy of this creature.'"
///
/// The board makes the entry clause a choice rather than a formality: the 1/1
/// Elf it names stands on its controller's side and the creature across the
/// table is offered beside it, which is what tells "any creature on the
/// battlefield" from a filter that only reads one half of it — while the
/// Forest in the same question is the permanent the word *creature* has to
/// decline. A 0/0 that copied nothing dies to CR 704.5f, so the 1/1 projected
/// onto the Mimic's own card is the copy itself; and the token that arrives on
/// its controller's next upkeep is a copy of the *creature* rather than of the
/// card, with the opponent's upkeep going by on the way against which "your
/// upkeep" is read.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn progenitor_mimic_enters_as_a_copy_and_makes_a_token_copy_of_itself_each_upkeep() {
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
                island(),
                quiet_creature(),
            ],
        )
        .hand(0, &[progenitor_mimic()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is on the table");
    let theirs = on_battlefield(&engine, p1, serra_angel()).expect("the Angel is on the table");
    let land = on_battlefield(&engine, p0, forest()).expect("a Forest is on the table");
    let elf_body = pt(&engine, host);
    assert_eq!(
        elf_body,
        (1, 1),
        "a printed 1/1, and the body the copy below is read against"
    );

    // Five Forests and an Island are exactly {4}{G}{U}. The Elf is named as the
    // printing kept back: it is the creature the Mimic is about to copy, and it
    // prints a `{T}: Add {G}` of its own that `tap_all_mana` would have taken.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "five Forests and one Island, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, progenitor_mimic());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{G}}{{U}} came out of the pool"
    );

    // The copy is chosen on the way in (CR 614.12a), so the question is asked
    // while the creature is entering and the answer decides what enters.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat casting it names the creature");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"any creature on the battlefield\" reaches both sides of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a permanent standing on the same board and no creature: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![host],
                players: vec![],
            },
        )
        .expect("the Elf was one of the creatures the copy offered");
    pass_until(&mut engine, stack_is_empty);

    let mimic = on_battlefield(&engine, p0, progenitor_mimic()).expect(
        "a copy is the Mimic's own card wearing another creature's characteristics, \
         and one that copied nothing would have died",
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, quiet_creature()).len(),
        1,
        "the copy left the card what it was: the Elf it named is still the only one"
    );
    assert_eq!(
        pt(&engine, mimic),
        elf_body,
        "the 0/0 Shapeshifter entered as a copy of what the question offered"
    );

    // "At the beginning of your upkeep": the Mimic's own turn has no upkeep
    // left in it, so the trigger belongs to p0's next one — and p1's upkeep on
    // the way there is the control for the word *your*.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let copies = tokens_of(&engine, p0);
    assert_eq!(
        copies.len(),
        1,
        "\"create a token that's a copy of this creature\" — one, off the \
         Mimic's controller's upkeep and not off the opponent's"
    );
    let token = copies[0];
    // `tokens_of` counted it as a token already: a permanent with no card
    // behind it. Not `GameObject::token`, which names the definition a token
    // was made from; a copy of a card-backed permanent carries that
    // permanent's, and the Mimic has none. What is left to say is *what* it
    // copied, and the name says it apart from the size below.
    let state = engine.state();
    let token_name = state
        .object(token)
        .expect("the copy is on the battlefield")
        .characteristics()
        .name;
    let elf_name = state
        .object(host)
        .expect("the Elf the Mimic copied is still on the table")
        .characteristics()
        .name;
    assert_eq!(
        state.names.get(token_name),
        state.names.get(elf_name),
        "what the upkeep made is a copy of the creature the Mimic *is*, and \
         not of the Progenitor Mimic card underneath it"
    );
    assert_eq!(
        pt(&engine, token),
        elf_body,
        "and it copies the creature the Mimic *is* — the 1/1 it entered as — \
         rather than the 0/0 the card prints"
    );
    assert_eq!(
        pt(&engine, mimic),
        elf_body,
        "with the Mimic itself still whatever it entered as"
    );
}
