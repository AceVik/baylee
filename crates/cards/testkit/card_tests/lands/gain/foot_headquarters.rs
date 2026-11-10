//! `cards/lands/gain/foot_headquarters.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Foot Headquarters prints three lines: it enters tapped, its controller
/// gains 1 life as it enters, and it taps for {W} or {B}.
///
/// The board is one land *played* rather than a seeded permanent, because
/// `starting_battlefield` places with `Cause::Setup` and never runs an
/// enters-tapped replacement — a test resting on it would watch an untapped
/// land and prove nothing. The life is read against the total from the moment
/// before the land drop, so the point is this land's gain and not the format's
/// number, and the menu is read as exactly two colours, so a `{T}: Add {W}`
/// could not pass for "or {B}". One turn cycle is walked first, because a land
/// that arrived tapped gives nothing until its controller's own untap step.
#[test]
fn foot_headquarters_enters_tapped_gains_a_life_and_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[foot_headquarters()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, foot_headquarters());

    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — a real land drop, and the replacement \
         still applied"
    );
    pass_until(&mut engine, |e| e.state().players[0].life != life_before);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\""
    );

    // A tapped land is no mana source until its controller's untap step
    // (CR 502.3), so the {T} line is read on a later turn of that seat.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so the activation below is about \
         the card and not about the turn it arrived in"
    );

    // No other source has ever produced anything, so whatever the pool holds
    // after this one activation came off this one land.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "an empty pool before the tap (CR 500.5 empties it with every step)"
    );
    activate(&mut engine, p0, foot_headquarters(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a colour question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours, and nothing else on the menu: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "both printed halves are offered: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, with no second source on the board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
