//! `cards/lands/gain/illegitimate_business.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Illegitimate Business prints three sentences, and the mana line is where a
/// body-only test would stop. It enters tapped, it gains its controller 1
/// life as it arrives, and it taps for `{B}` or `{G}` — so the scenario plays
/// it, reads the gain off the enters trigger, and then walks a whole turn
/// cycle, because a permanent that entered tapped has no `{T}` to pay with
/// until its controller's next untap step (CR 502.3) and pressing the
/// ability earlier would fail for a reason that is not the card's. The colour
/// question is the third sentence: exactly two options and not "any colour",
/// with the pool afterwards holding the one colour that was named.
#[test]
fn illegitimate_business_enters_tapped_gains_a_life_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(7, forest())
        .hand(0, &[illegitimate_business()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, illegitimate_business());
    assert!(
        is_tapped(&engine, land),
        "the printed replacement makes it enter tapped (CR 614.1)"
    );

    // The life arrives off the enters trigger and not off the land drop: the
    // permanent is already on the battlefield and tapped while this is still
    // waiting to resolve.
    pass_until(&mut engine, |e| e.state().players[0].life == 21);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\""
    );

    // Nothing untaps it here, which is why the mana line needs a turn: for
    // the rest of p0's own turn the land is a permanent with no payable
    // `{T}` at all.
    assert!(is_tapped(&engine, land), "and it stays tapped all turn");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so {{T}} is payable again"
    );

    // Ability 0 is the enters trigger; the mana ability the card prints is
    // index 1, and a printed mana ability is an ordinary `(source, index)`
    // entry in `LegalActions::abilities` however mana-like it is (CR 605.1) —
    // a land with no basic land type has no CR 305.6 shortcut to live under.
    activate(&mut engine, p0, illegitimate_business(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours and not \"any colour\": {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "exactly the pair the card prints: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "the other half of the choice was not produced"
    );
    assert_eq!(pool.total(), 1, "one tap, one mana, off an empty pool");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
