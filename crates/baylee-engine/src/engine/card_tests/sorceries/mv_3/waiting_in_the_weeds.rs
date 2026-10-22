//! `cards/sorceries/mv_3/waiting_in_the_weeds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn waiting_in_the_weeds() -> CardIndex {
    card_index("5a61d51e-1a3f-49f6-9a94-a708dd6f69df")
}

/// Waiting in the Weeds (`{1}{G}{G}`, `Coverage::Partial`): creates a 1/1 green
/// Cat creature token for each untapped Forest you control.
#[test]
fn waiting_in_the_weeds_creates_cat_tokens_for_untapped_forests_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[waiting_in_the_weeds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap 3 Forests to cast Waiting in the Weeds, leaving 2 untapped Forests -> 2 1/1 green Cat tokens.
    let forests = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| {
        id == forests[0] || id == forests[1] || id == forests[2]
    });
    cast_with_floating(&mut engine, p0, waiting_in_the_weeds());
    pass_until(&mut engine, stack_is_empty);

    let cats = tokens_of(&engine, p0);
    assert_eq!(cats.len(), 2);
    assert_eq!(pt(&engine, cats[0]), (1, 1));
}
