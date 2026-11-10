//! `cards/sorceries/mv_5/dosan_s_oldest_chant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "59aeeb18-d263-426f-aebc-0f687b09801b"

/// Dosan's Oldest Chant is `{4}{G}` for two sentences with nothing between
/// them: "You gain 6 life" and "Draw a card". Neither number is visible in the
/// card file, so the board makes each one exact: six is neither the five mana
/// paid nor one per Forest tapped, and the drawn card is named as the object
/// that was on top of the library before the cast, because a spell that merely
/// removed the top of the library would fill the same count while a hand that
/// stayed empty went unnoticed. Five Forests are exactly the printed cost, so
/// an empty pool afterwards says the `{4}{G}` was really paid rather than
/// announced.
#[test]
fn dosans_oldest_chant_pays_five_mana_for_six_life_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[dosan_s_oldest_chant()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the five untapped Forests: with nothing floating the cost is
    // unpayable, so the Chant is not among the castable cards at all.
    let card = in_hand(&engine, p0, dosan_s_oldest_chant()).expect("the Chant is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}{{G}}, so the Chant is not offered: {:?}",
        legal.castable
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests in the pool, which is exactly the printed cost"
    );
    cast_with_floating(&mut engine, p0, dosan_s_oldest_chant());

    assert!(
        !stack_is_empty(&engine),
        "a sorcery uses the stack, so the Chant is waiting to resolve"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has been gained while it is still there"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{G}} came out of the pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        26,
        "\"You gain 6 life.\" — six, and never one per mana spent or one per \
         Forest tapped"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that cast the Chant, not to the opponent"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top of the library, now in hand — \
         a spell that only emptied the library would leave the hand short"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Chant left the hand to be cast (CR 601.2a) and the draw replaced \
         it, so the hand is the size it was"
    );
    assert!(
        in_graveyard(&engine, p0, dosan_s_oldest_chant()).is_some(),
        "and the sorcery itself is in its owner's graveyard, which is where a \
         resolved spell goes"
    );
}
