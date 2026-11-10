//! `cards/lands/gates/rakdos_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rakdos Guildgate prints two sentences, and both are played rather than
/// read: "This land enters tapped", and "{T}: Add {B} or {R}". The entry is
/// taken off the permanent the moment it arrives — the whole point of the
/// card is that the land is not usable the turn it is played, and a board
/// seeded with `starting_battlefield` would never show it, because that
/// placement is not an entry (no replacement effect looks at it). The mana
/// line is therefore read one turn later, after the untap step that is its
/// controller's; the colour question is asserted as two options and no
/// third, and the mana that lands is the one that was named, off a pool
/// that was empty before the tap.
#[test]
fn rakdos_guildgate_enters_tapped_and_taps_for_black_or_red_the_turn_after() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[rakdos_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = play_land(&mut engine, p0, rakdos_guildgate());
    assert!(
        on_battlefield(&engine, p0, rakdos_guildgate()).is_some(),
        "the land reached the battlefield"
    );
    assert!(
        entered_tapped(&engine, gate),
        "\"This land enters tapped\" — the printed entry modifier, which \
         only a real `PlayLand` puts on the board"
    );

    // A turn each way: the untap step that stands the Gate back up belongs
    // to its controller, so nothing about its {T} is readable before then.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gate),
        "the untap step stood it back up, so the tap below is a choice and \
         not a refusal waiting to happen"
    );

    // The Gate's whole price is its own {T}: with an empty pool, whatever
    // arrives afterwards has exactly one possible source on this board.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so the mana asserted below came off the Gate"
    );
    activate(&mut engine, p0, rakdos_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the Gate names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "both printed colours are on offer: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and only those two — a colourless option would be a different card, \
         and a Gate that were also a Mountain would offer red twice: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and not a second colour for the one tap"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
}
