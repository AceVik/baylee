//! `cards/lands/tapland/shivan_oasis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shivan Oasis prints two sentences — "This land enters tapped" and "{T}: Add
/// {R} or {G}" — and the board is a bare table so that neither can be read off
/// something else: with no other permanent in play the tap status on arrival can
/// only come from the printed entry modifier (CR 614.1), and with an empty pool
/// the one mana that lands afterwards can only have come off the land's own tap.
/// The turn cycle is the other half of the first sentence: a land whose `{T}` is
/// already spent offers nothing at all on the turn it arrives, and is standing
/// untapped and offering both colours on the next one.
#[test]
fn shivan_oasis_enters_tapped_and_taps_for_red_or_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest()).hand(0, &[shivan_oasis()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shivan_oasis());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and nothing else on this bare board \
         could have tapped it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land drop costs no mana, so the pool is empty before the tap"
    );

    // The tap symbol is already spent, so the mana line is not offered at all:
    // `legal.abilities` is filtered by what can actually be paid (CR 601.2h).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a land that entered tapped has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so the tap was the entry modifier and not a \
         permanent state"
    );

    // "{T}: Add {R} or {G}" is a mana ability the card *prints*, so it carries
    // an index and its whole price is the tap symbol — no mana is needed first.
    activate(&mut engine, p0, shivan_oasis(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}} or {{G}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "the two colours the card prints, and no third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid the tap itself");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "one green, as named");
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "one mana of one colour, not one of each"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
