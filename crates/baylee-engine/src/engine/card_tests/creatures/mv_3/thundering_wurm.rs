//! `cards/creatures/mv_3/thundering_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thundering Wurm — {2}{G}, a printed 4/4 — "When this creature enters,
/// sacrifice it unless you discard a land card."
///
/// The trigger is played rather than read: three Forests pay for the Wurm,
/// the engine asks its controller for the price when the trigger resolves,
/// and the Wurm is a 4/4 on the battlefield afterwards while a land card has
/// moved from the hand to the graveyard. The two failure modes are separated
/// by the counts below — a trigger that never asked leaves the hand one card
/// fuller, one that sacrificed unconditionally leaves no Wurm — and the land
/// in the graveyard is what says the price read "a land card" rather than
/// "a card".
#[test]
fn thundering_wurm_buys_itself_with_a_land_card_from_the_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[thundering_wurm(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The price has to be payable on this board or the engine never asks its
    // question at all: a land card is named into the hand so that the trigger
    // really asks, instead of resolving straight to the sacrifice.
    assert!(
        in_hand(&engine, p0, thundering_wurm()).is_some(),
        "the Wurm is in hand to be cast"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "and a land card is in hand to be given up for it"
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0,
        "nothing has been given up yet"
    );

    // {2}{G} off the three Forests, and the entry trigger is what the driver
    // then meets — the question is answered with the price.
    cast_from_hand(&mut engine, p0, thundering_wurm());
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the entry trigger's question is answerable and the game comes back \
         to a quiet priority"
    );

    let wurm = on_battlefield(&engine, p0, thundering_wurm())
        .expect("the Wurm kept itself by paying the price");
    assert_eq!(pt(&engine, wurm), (4, 4), "the body the card prints");
    assert!(
        in_graveyard(&engine, p0, thundering_wurm()).is_none(),
        "and it was not sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "\"unless you discard a land card\" — the price was a land card, and \
         it is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "exactly one card was given up: one discard and no more"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2,
        "the Wurm left the hand to be cast and one land card left it as the \
         price; a trigger that never asked would stop at one"
    );
}
