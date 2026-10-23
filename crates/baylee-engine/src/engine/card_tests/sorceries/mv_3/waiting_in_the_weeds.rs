//! `cards/sorceries/mv_3/waiting_in_the_weeds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn waiting_in_the_weeds() -> CardIndex {
    card_index("5a61d51e-1a3f-49f6-9a94-a708dd6f69df")
}

/// Waiting in the Weeds (`{1}{G}{G}`, `Coverage::Partial`): each player
/// creates a 1/1 green Cat for each untapped Forest they control. P0 taps
/// three of five Forests to cast it and so counts two; P1 controls three
/// Forests and a Plains and so counts three, and the Plains is not a Forest.
#[test]
fn waiting_in_the_weeds_gives_each_player_cats_for_their_own_untapped_forests() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .battlefield(1, &[forest(), forest(), forest(), plains()])
        .hand(0, &[waiting_in_the_weeds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap 3 Forests to cast Waiting in the Weeds, leaving 2 untapped.
    let forests = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| {
        id == forests[0] || id == forests[1] || id == forests[2]
    });
    cast_with_floating(&mut engine, p0, waiting_in_the_weeds());
    pass_until(&mut engine, stack_is_empty);

    let mine = tokens_of(&engine, p0);
    assert_eq!(mine.len(), 2, "two untapped Forests of P0's own");
    assert_eq!(pt(&engine, mine[0]), (1, 1));
    let theirs = tokens_of(&engine, p1);
    assert_eq!(
        theirs.len(),
        3,
        "three Forests of P1's own, and the Plains does not count"
    );
    assert_eq!(pt(&engine, theirs[0]), (1, 1));
}
