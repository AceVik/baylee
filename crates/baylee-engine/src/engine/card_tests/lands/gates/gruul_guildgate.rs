//! `cards/lands/gates/gruul_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gruul Guildgate — Land — Gate: "This land enters tapped." and "{T}: Add
/// {R} or {G}."
///
/// The tapped entry only shows up through a real `PlayLand`: a permanent
/// seeded by `starting_battlefield` is placed with `Cause::Setup`, no
/// replacement effect looks at it, and a board built that way would read the
/// Gate untapped whatever the card says. The mana line is read a turn later,
/// because its own untap step is the first moment a land that arrives tapped
/// can pay a `{T}` at all (CR 502.3). Both colours are taken, in two turns,
/// and the colour must be asked for each time — a Gate that quietly made one
/// of them would pass everything short of that.
#[test]
fn gruul_guildgate_enters_tapped_and_taps_for_either_of_its_two_colors() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(51, forest())
        .hand(0, &[gruul_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = play_land(&mut engine, p0, gruul_guildgate());
    assert!(
        entered_tapped(&engine, gate),
        "\"This land enters tapped\" — and it was *played*, so the \
         replacement effect had an entry to look at"
    );

    // Two hops rather than one: a land that enters tapped gives nothing in
    // the turn it arrived in, and `reach_their_main_phase(p0)` would answer
    // "you are already there" from the main phase this started in.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(!is_tapped(&engine, gate), "the untap step stood it back up");

    // Ability 0 is the printed "{T}: Add {R} or {G}"; nothing is tapped on
    // the way in, because the whole price is the Gate's own tap symbol.
    activate(&mut engine, p0, gruul_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the Gate names the colour");
    assert_eq!(
        options.len(),
        2,
        "\"or\" is two colours and no more: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "{{R}} or {{G}}, with neither missing: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");

    // The other half of the sentence. A second turn is the price of a second
    // tap, and the pool the first tap filled is emptied when its phase ended
    // (CR 500.5), so what is read below is this tap and nothing else.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes a third turn");
    assert!(!is_tapped(&engine, gate), "and the Gate untaps again");
    activate(&mut engine, p0, gruul_guildgate(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "the second tap asks the same question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 2, "\"or\", both times: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the other colour, on the other tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not the colour the first tap took"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
